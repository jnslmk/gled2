use crate::{
    midi::state::MidiState,
    output_state::ProjectState,
    storage::{
        asset::{
            Asset,
            midi_controller::{MidiController, MidiControllerMapping},
            project::Project,
            scene::color::SceneInstanceColor,
        },
        collections::Collections,
    },
};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, unbounded};
use midir::MidiOutputConnection;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::thread::spawn;

mod input_handling;
mod output_eval;
mod scene_state;

#[derive(Default, Clone)]
pub struct RuntimeTestState {
    pub selected_output_port: Option<String>,
    pub value_output_overrides: HashMap<usize, u8>,
    pub scene_color_value_overrides: HashMap<String, u8>,
}

static RUNTIME_TEST_STATES: OnceLock<
    StdMutex<HashMap<crate::storage::asset_id::AssetId<MidiController>, RuntimeTestState>>,
> = OnceLock::new();

pub(super) fn runtime_test_states(
) -> &'static StdMutex<HashMap<crate::storage::asset_id::AssetId<MidiController>, RuntimeTestState>> {
    RUNTIME_TEST_STATES.get_or_init(|| StdMutex::new(HashMap::new()))
}

pub fn set_controller_test_state(
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    state: Option<RuntimeTestState>,
) {
    let mut states = runtime_test_states()
        .lock()
        .expect("runtime test states lock poisoned");
    if let Some(state) = state {
        states.insert(controller_id, state);
    } else {
        states.remove(&controller_id);
    }
}

pub fn clear_all_test_states() {
    runtime_test_states()
        .lock()
        .expect("runtime test states lock poisoned")
        .clear();
}

#[derive(Clone)]
pub struct RuntimeBus {
    subscribers: Arc<Mutex<Vec<Sender<MidiRuntimeSnapshot>>>>,
}

pub struct Runtime {
    receiver: Receiver<MidiRuntimeSnapshot>,
    snapshot: MidiRuntimeSnapshot,
}

#[derive(Default, Clone)]
pub(super) struct MidiRuntimeSnapshot {
    pub(super) mappings_by_controller: HashMap<RuntimeControllerKey, ActiveRuntimeMapping>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum RuntimeControllerKey {
    PortName(String),
    TestOnly(crate::storage::asset_id::AssetId<MidiController>),
}

#[derive(Clone)]
pub(super) struct ActiveRuntimeMapping {
    pub(super) controller_id: crate::storage::asset_id::AssetId<MidiController>,
    pub(super) mapping: MidiControllerMapping,
}

impl Runtime {
    fn refresh_snapshot(&mut self) {
        while let Ok(Some(snapshot)) = self.receiver.try_recv() {
            self.snapshot = snapshot;
        }
    }

    pub fn handle_input(&mut self, port_name: &str, message: &[u8]) -> bool {
        self.refresh_snapshot();
        input_handling::handle_input_from_snapshot(&self.snapshot, port_name, message)
    }

    pub fn send_output(
        &mut self,
        port_name: &str,
        state: &MidiState,
        connection: &mut MidiOutputConnection,
    ) -> bool {
        self.refresh_snapshot();
        output_eval::send_output_from_snapshot(
            &self.snapshot.mappings_by_controller,
            port_name,
            state,
            connection,
        )
    }
}

impl RuntimeBus {
    pub fn runtime(&self) -> Runtime {
        let (sender, receiver) = unbounded();
        self.subscribers.lock().push(sender);
        Runtime {
            receiver,
            snapshot: MidiRuntimeSnapshot::default(),
        }
    }
}

impl Default for RuntimeBus {
    fn default() -> Self {
        let subscribers: Arc<Mutex<Vec<Sender<MidiRuntimeSnapshot>>>> =
            Arc::new(Mutex::new(Vec::new()));
        let subscribers_for_thread = Arc::clone(&subscribers);

        spawn(move || {
            #[cfg(feature = "profiling")]
            profiling::register_thread!("midi:runtime");

            let state_receiver = crate::output_state::new_receiver();
            let mut collections = Collections::default();

            loop {
                let Ok(state) = state_receiver.recv() else {
                    continue;
                };
                collections.update();
                let snapshot = snapshot_from_project_state(&state, &collections);
                subscribers_for_thread
                    .lock()
                    .retain(|subscriber| subscriber.send(snapshot.clone()).is_ok());
            }
        });

        Self { subscribers }
    }
}

fn snapshot_from_project_state(state: &ProjectState, collections: &Collections) -> MidiRuntimeSnapshot {
    let mut mappings_by_controller = HashMap::new();
    let mut included_controller_ids = HashSet::new();

    if let Some(project) = state.project.as_ref() {
        for (controller_key, selection) in &project.midi_active_mappings {
            let Some(controller_id) = selection else {
                continue;
            };
            let Some(controller) = Asset::<MidiController>::get(*controller_id, collections) else {
                continue;
            };

            mappings_by_controller.insert(
                RuntimeControllerKey::PortName(controller_key.clone()),
                ActiveRuntimeMapping {
                    controller_id: *controller_id,
                    mapping: controller.data.mapping.clone(),
                },
            );
            included_controller_ids.insert(*controller_id);
        }
    }

    // Controllers not routed via External Devices still participate in Test Device mode.
    for controller in Asset::<MidiController>::all(collections) {
        if included_controller_ids.contains(&controller.id) {
            continue;
        }

        mappings_by_controller.insert(
            RuntimeControllerKey::TestOnly(controller.id),
            ActiveRuntimeMapping {
                controller_id: controller.id,
                mapping: controller.data.mapping.clone(),
            },
        );
    }

    MidiRuntimeSnapshot {
        mappings_by_controller,
    }
}

pub fn selected_scene_color(
    project: Option<&mut Project>,
    fallback: SceneInstanceColor,
    selected: crate::storage::asset::scene::grid::GridLocation,
) -> SceneInstanceColor {
    project
        .and_then(|project| project.get_scenes_instance(&selected).map(|scene| scene.color))
        .unwrap_or(fallback)
}
