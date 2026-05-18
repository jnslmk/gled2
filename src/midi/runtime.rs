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

/// UI -> Runtime test command for MIDI output testing
#[derive(Debug, Clone)]
pub enum TestCommand {
    /// Send a MIDI test output: (status, data1, value)
    SendOutput { status: u8, data1: u8, value: u8 },
    /// Clear a MIDI test output by (status, data1)
    ClearOutput { status: u8, data1: u8 },
    /// Set the test device port name
    SetTestDevice { port_name: String },
    /// Unset test device (stop all test sending)
    UnsetTestDevice,
}

fn create_test_command_channel() -> (Sender<TestCommand>, Arc<Mutex<Receiver<TestCommand>>>) {
    let (sender, receiver) = unbounded::<TestCommand>();
    (sender, Arc::new(Mutex::new(receiver)))
}

#[derive(Default, Clone)]
pub struct RuntimeTestState {
    pub selected_output_port: Option<String>,
    pub scene_color_value_overrides: HashMap<String, u8>,
    pub value_preview_binding_indices: HashSet<usize>,
    pub preview_mapping: Option<MidiControllerMapping>,
}

/// Runtime execution state for test device output.
/// Maintained by test command processing in output_eval loop.
#[derive(Debug, Clone, Default)]
pub(super) struct RuntimeTestExecutionState {
    /// Selected test device port name
    pub selected_test_device: Option<String>,
    /// Active test outputs: (status, data1) -> value
    pub active_outputs: HashMap<(u8, u8), u8>,
}

static RUNTIME_TEST_STATES: OnceLock<
    StdMutex<HashMap<crate::storage::asset_id::AssetId<MidiController>, RuntimeTestState>>,
> = OnceLock::new();

static RUNTIME_TEST_EXECUTION_STATE: OnceLock<StdMutex<RuntimeTestExecutionState>> =
    OnceLock::new();

pub(super) fn runtime_test_states()
-> &'static StdMutex<HashMap<crate::storage::asset_id::AssetId<MidiController>, RuntimeTestState>> {
    RUNTIME_TEST_STATES.get_or_init(|| StdMutex::new(HashMap::new()))
}

pub(super) fn runtime_test_execution_state() -> &'static StdMutex<RuntimeTestExecutionState> {
    RUNTIME_TEST_EXECUTION_STATE.get_or_init(|| StdMutex::new(RuntimeTestExecutionState::default()))
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
    test_command_receiver: Arc<Mutex<Receiver<TestCommand>>>,
}

pub struct Runtime {
    receiver: Receiver<MidiRuntimeSnapshot>,
    snapshot: MidiRuntimeSnapshot,
    test_command_receiver: Arc<Mutex<Receiver<TestCommand>>>,
    last_sent_outputs: HashMap<(u8, u8), u8>,
    last_input_values: HashMap<(u8, u8), u8>,
}

#[derive(Default, Clone)]
pub(super) struct MidiRuntimeSnapshot {
    pub(super) mappings_by_controller: HashMap<RuntimeControllerKey, ActiveRuntimeMapping>,
    pub(super) scene_locations_row_major: Vec<crate::storage::asset::scene::grid::GridLocation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum RuntimeControllerKey {
    PortName(String),
    TestOnly(crate::storage::asset_id::AssetId<MidiController>),
}

#[derive(Clone)]
pub(super) struct ActiveRuntimeMapping {
    pub(super) _controller_id: crate::storage::asset_id::AssetId<MidiController>,
    pub(super) mapping: MidiControllerMapping,
}

impl Runtime {
    fn refresh_snapshot(&mut self) {
        while let Ok(Some(snapshot)) = self.receiver.try_recv() {
            self.snapshot = snapshot;
        }
    }

    fn process_test_commands(&mut self) {
        let mut exec_state = runtime_test_execution_state()
            .lock()
            .expect("test execution state lock poisoned");

        let test_receiver = self.test_command_receiver.lock();
        while let Ok(Some(cmd)) = test_receiver.try_recv() {
            match cmd {
                TestCommand::SendOutput {
                    status,
                    data1,
                    value,
                } => {
                    let old = exec_state.active_outputs.insert((status, data1), value);
                    if old != Some(value)
                        && let Some(port) = exec_state.selected_test_device.as_deref()
                    {
                        crate::midi::monitor::push_event(
                            port,
                            "test output",
                            &[status, data1, value],
                        );
                    }
                }
                TestCommand::ClearOutput { status, data1 } => {
                    if exec_state.active_outputs.remove(&(status, data1)).is_some()
                        && let Some(port) = exec_state.selected_test_device.as_deref()
                    {
                        crate::midi::monitor::push_event(port, "test clear", &[status, data1]);
                    }
                }
                TestCommand::SetTestDevice { port_name } => {
                    exec_state.selected_test_device = Some(port_name);
                }
                TestCommand::UnsetTestDevice => {
                    exec_state.selected_test_device = None;
                    exec_state.active_outputs.clear();
                }
            }
        }
    }

    pub fn handle_input(&mut self, port_name: &str, message: &[u8]) -> bool {
        self.refresh_snapshot();
        let handled =
            input_handling::handle_input_from_snapshot(&self.snapshot, port_name, message);
        if handled && message.len() == 3 {
            self.last_input_values
                .insert((message[0], message[1]), message[2]);
        }
        handled
    }

    pub fn send_output(
        &mut self,
        port_name: &str,
        state: &MidiState,
        connection: &mut MidiOutputConnection,
    ) -> bool {
        self.refresh_snapshot();
        self.process_test_commands();
        output_eval::send_output_from_snapshot(
            &self.snapshot,
            port_name,
            state,
            connection,
            &mut self.last_sent_outputs,
            &self.last_input_values,
        )
    }
}

impl RuntimeBus {
    pub fn new() -> (Self, Sender<TestCommand>) {
        let (test_command_sender, test_command_receiver) = create_test_command_channel();
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

        (
            Self {
                subscribers,
                test_command_receiver,
            },
            test_command_sender,
        )
    }

    pub fn runtime(&self) -> Runtime {
        let (sender, receiver) = unbounded();
        self.subscribers.lock().push(sender);

        Runtime {
            receiver,
            snapshot: MidiRuntimeSnapshot::default(),
            test_command_receiver: Arc::clone(&self.test_command_receiver),
            last_sent_outputs: HashMap::new(),
            last_input_values: HashMap::new(),
        }
    }
}

impl Default for RuntimeBus {
    fn default() -> Self {
        let (runtime_bus, _test_command_sender) = Self::new();
        runtime_bus
    }
}

fn snapshot_from_project_state(
    state: &ProjectState,
    collections: &Collections,
) -> MidiRuntimeSnapshot {
    let mut mappings_by_controller = HashMap::new();
    let mut included_controller_ids = HashSet::new();
    let mut scene_locations_row_major = Vec::new();

    if let Some(project) = state.project.as_ref() {
        scene_locations_row_major = project.scenes_instances_grid.keys().copied().collect();
        scene_locations_row_major.sort_by_key(|location| (location.row, location.col));
    }

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
                    _controller_id: *controller_id,
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
                _controller_id: controller.id,
                mapping: controller.data.mapping.clone(),
            },
        );
    }

    MidiRuntimeSnapshot {
        mappings_by_controller,
        scene_locations_row_major,
    }
}

pub fn selected_scene_color(
    project: Option<&mut Project>,
    fallback: SceneInstanceColor,
    selected: crate::storage::asset::scene::grid::GridLocation,
) -> SceneInstanceColor {
    project
        .and_then(|project| {
            project
                .get_scenes_instance(&selected)
                .map(|scene| scene.color)
        })
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_enum_creation() {
        let cmd_send = TestCommand::SendOutput {
            status: 144,
            data1: 60,
            value: 100,
        };
        assert!(matches!(
            cmd_send,
            TestCommand::SendOutput {
                status: 144,
                data1: 60,
                value: 100
            }
        ));

        let cmd_clear = TestCommand::ClearOutput {
            status: 144,
            data1: 60,
        };
        assert!(matches!(
            cmd_clear,
            TestCommand::ClearOutput {
                status: 144,
                data1: 60
            }
        ));

        let cmd_set = TestCommand::SetTestDevice {
            port_name: "Device1".to_string(),
        };
        assert!(matches!(cmd_set, TestCommand::SetTestDevice { .. }));

        let cmd_unset = TestCommand::UnsetTestDevice;
        assert!(matches!(cmd_unset, TestCommand::UnsetTestDevice));
    }

    #[test]
    fn test_runtime_test_execution_state_active_outputs() {
        let state = runtime_test_execution_state();
        let mut exec_state = state.lock().expect("test execution state lock poisoned");

        // Initially empty
        assert!(exec_state.active_outputs.is_empty());
        assert!(exec_state.selected_test_device.is_none());

        // Add outputs
        exec_state.active_outputs.insert((144, 60), 100);
        exec_state.active_outputs.insert((176, 7), 64);
        assert_eq!(exec_state.active_outputs.len(), 2);

        // Update value
        exec_state.active_outputs.insert((144, 60), 127);
        assert_eq!(exec_state.active_outputs.get(&(144, 60)), Some(&127));

        // Remove output
        exec_state.active_outputs.remove(&(144, 60));
        assert_eq!(exec_state.active_outputs.len(), 1);

        // Clear all
        exec_state.active_outputs.clear();
        assert!(exec_state.active_outputs.is_empty());
    }

    #[test]
    fn test_runtime_test_execution_state_device_management() {
        let state = runtime_test_execution_state();
        let mut exec_state = state.lock().expect("test execution state lock poisoned");

        // Clear from any previous test
        exec_state.selected_test_device = None;
        exec_state.active_outputs.clear();

        // Set device
        exec_state.selected_test_device = Some("MidiOut1".to_string());
        assert_eq!(
            exec_state.selected_test_device,
            Some("MidiOut1".to_string())
        );

        // Add outputs while device is set
        exec_state.active_outputs.insert((144, 60), 100);
        assert_eq!(exec_state.active_outputs.len(), 1);

        // Clear device and outputs
        exec_state.selected_test_device = None;
        exec_state.active_outputs.clear();
        assert!(exec_state.selected_test_device.is_none());
        assert!(exec_state.active_outputs.is_empty());
    }
}
