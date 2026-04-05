use crate::{
    midi::state::MidiState,
    output_state::ProjectState,
    storage::{
        asset::{
            Asset,
            midi_controller::{
                MidiColorSource, MidiController,
                MidiControllerMapping, MidiInputAction, MidiOutputBindingKind,
                MidiSceneTarget, MidiValueSource,
            },
            project::Project,
            project::scene_instance_path::{
                SceneInstanceUnion, grid_scene_instance_index, quick_scene_instance_index,
            },
            scene::color::SceneInstanceColor,
        },
        collections::Collections,
    },
    ui::action::UiAction,
};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, unbounded};
use midir::MidiOutputConnection;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::thread::spawn;

#[derive(Default, Clone)]
pub struct RuntimeTestState {
    pub selected_output_port: Option<String>,
    pub value_output_overrides: HashMap<usize, u8>,
    pub scene_color_value_overrides: HashMap<String, u8>,
}

static RUNTIME_TEST_STATES: OnceLock<StdMutex<HashMap<crate::storage::asset_id::AssetId<MidiController>, RuntimeTestState>>> = OnceLock::new();

fn runtime_test_states(
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
struct MidiRuntimeSnapshot {
    mappings_by_controller: HashMap<RuntimeControllerKey, ActiveRuntimeMapping>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum RuntimeControllerKey {
    PortName(String),
    TestOnly(crate::storage::asset_id::AssetId<MidiController>),
}

#[derive(Clone)]
struct ActiveRuntimeMapping {
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    mapping: MidiControllerMapping,
}

impl Runtime {
    fn refresh_snapshot(&mut self) {
        while let Ok(Some(snapshot)) = self.receiver.try_recv() {
            self.snapshot = snapshot;
        }
    }

    pub fn handle_input(&mut self, port_name: &str, message: &[u8]) -> bool {
        self.refresh_snapshot();
        handle_input_from_snapshot(&self.snapshot, port_name, message)
    }

    pub fn send_output(
        &mut self,
        port_name: &str,
        state: &MidiState,
        connection: &mut MidiOutputConnection,
    ) -> bool {
        self.refresh_snapshot();
        send_output_from_snapshot(&self.snapshot, port_name, state, connection)
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

fn handle_input_from_snapshot(
    snapshot: &MidiRuntimeSnapshot,
    port_name: &str,
    message: &[u8],
) -> bool {
    if message.len() != 3 {
        return false;
    }

    let mapping = resolve_mapping_for_port(&snapshot.mappings_by_controller, port_name);
    let Some(mapping) = mapping else {
        return false;
    };

    let status = message[0];
    let data1 = message[1];
    let value = message[2];
    let mut handled_any = false;

    for binding in &mapping.mapping.input_bindings {
        if !trigger_matches(&binding.trigger, status, data1) {
            continue;
        }

        let action = match binding.action {
            MidiInputAction::Tap => Some(UiAction::Tap),
            MidiInputAction::SetMainDimmer => {
                Some(UiAction::SetMainDimmer(f32::from(value) / 127.0))
            }
            MidiInputAction::SetBlackout => Some(UiAction::SetBlackout(value > 0)),
            MidiInputAction::SetSpeedAdd => {
                let delta = if value > 64 {
                    f32::from(value) - 128.0
                } else {
                    f32::from(value)
                };
                Some(UiAction::SpeedAdd(delta))
            }
            MidiInputAction::SetSpeedMultiply => {
                Some(UiAction::SpeedMultiply(if value > 63 { 2.0 } else { 0.5 }))
            }
            MidiInputAction::SelectScene { ref target } => match target {
                MidiSceneTarget::Selected => None,
                _ => scene_target_to_union(target).map(UiAction::SelectScene),
            },
            MidiInputAction::ToggleSceneActive { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::ToggleSelectedSceneActive),
                _ => scene_target_to_union(target).map(UiAction::ToggleSceneActive),
            },
            MidiInputAction::SetSceneActive { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneActive(value > 0)),
                _ => scene_target_to_union(target)
                    .map(|target| UiAction::SetSceneActive(target, value > 0)),
            },
            MidiInputAction::SetSceneOpacity { ref target } => match target {
                MidiSceneTarget::Selected => {
                    Some(UiAction::SetSelectedSceneOpacity(f32::from(value) / 127.0))
                }
                _ => scene_target_to_union(target)
                    .map(|target| UiAction::SetSceneOpacity(target, f32::from(value) / 127.0)),
            },
            MidiInputAction::SetSceneInputDimmer { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneInputDimmer(
                    f32::from(value) / 127.0,
                )),
                _ => scene_target_to_union(target)
                    .map(|target| UiAction::SetSceneInputDimmer(target, f32::from(value) / 127.0)),
            },
            MidiInputAction::SetSceneBeatOffset { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneBeatOffset(
                    f32::from(value) / 127.0,
                )),
                _ => scene_target_to_union(target)
                    .map(|target| UiAction::SetSceneBeatOffset(target, f32::from(value) / 127.0)),
            },
            MidiInputAction::SetSceneIgnoreMainDimmer { ref target } => match target {
                MidiSceneTarget::Selected => {
                    Some(UiAction::SetSelectedSceneIgnoreMainDimmer(value > 63))
                }
                _ => scene_target_to_union(target)
                    .map(|target| UiAction::SetSceneIgnoreMainDimmer(target, value > 63)),
            },
            MidiInputAction::SetSceneSetOffsetOnFlash { ref target } => match target {
                MidiSceneTarget::Selected => {
                    Some(UiAction::SetSelectedSceneSetOffsetOnFlash(value > 63))
                }
                _ => scene_target_to_union(target)
                    .map(|target| UiAction::SetSceneSetOffsetOnFlash(target, value > 63)),
            },
            MidiInputAction::SetSceneEffectSettingF32 {
                ref target,
                effect_index,
                setting_index,
            } => match target {
                MidiSceneTarget::Selected => {
                    Some(UiAction::SetSelectedSceneEffectAnimationConfigF32(
                        effect_index as usize,
                        setting_index as usize,
                        f32::from(value) / 127.0,
                    ))
                }
                _ => scene_target_to_union(target).map(|target| {
                    UiAction::SetSceneEffectAnimationConfigF32(
                        target,
                        effect_index as usize,
                        setting_index as usize,
                        f32::from(value) / 127.0,
                    )
                }),
            },
        };

        if let Some(action) = action {
            action.enqueue();
            handled_any = true;
        }
    }

    handled_any
}

fn send_output_from_snapshot(
    snapshot: &MidiRuntimeSnapshot,
    port_name: &str,
    state: &MidiState,
    connection: &mut MidiOutputConnection,
) -> bool {
    let test_states = runtime_test_states()
        .lock()
        .expect("runtime test states lock poisoned")
        .clone();
    let mut sent_any = false;

    for (controller_key, mapping) in &snapshot.mappings_by_controller {
        let test_state = test_states.get(&mapping.controller_id).cloned().unwrap_or_default();
        let selected_test_port = test_state
            .selected_output_port
            .as_deref()
            .is_some_and(|port| port == port_name);
        let routed_to_port = matches!(
            controller_key,
            RuntimeControllerKey::PortName(name) if name == port_name
        );
        if !routed_to_port && !selected_test_port {
            continue;
        }

        for (binding_index, binding) in mapping.mapping.output_bindings.iter().enumerate() {
        match &binding.kind {
            MidiOutputBindingKind::Value(output) => {
                let mut midi_value = match output.source {
                    MidiValueSource::SelectedSceneOpacity => {
                        scale_to_range(state.selected_scene_opacity, output.min, output.max)
                    }
                    MidiValueSource::SelectedSceneInputDimmer => scale_to_range(
                        state.selected_scene_input_dimmer,
                        output.min,
                        output.max,
                    ),
                    MidiValueSource::SelectedSceneBeatOffset => {
                        scale_to_range(state.selected_scene_beat_offset, output.min, output.max)
                    }
                    MidiValueSource::SelectedSceneIgnoreMainDimmer => binary_output_value(
                        state.selected_scene_ignore_main_dimmer,
                        output.active_value,
                    ),
                    MidiValueSource::SelectedSceneSetOffsetOnFlash => binary_output_value(
                        state.selected_scene_set_offset_on_flash,
                        output.active_value,
                    ),
                    MidiValueSource::MainDimmer => {
                        scale_to_range(state.main_dimmer, output.min, output.max)
                    }
                    MidiValueSource::BeatFlankPulse {
                        start_beat,
                        end_beat,
                    } => binary_output_value(
                        beat_range_contains(state.beat_progression.rem_euclid(4.0), start_beat, end_beat),
                        output.active_value,
                    ),
                    MidiValueSource::Blackout { inverted, blink } => binary_output_value(
                        blackout_output_active(state, inverted, blink),
                        output.active_value,
                    ),
                    MidiValueSource::SceneOpacity { ref target } => {
                        scale_to_range(scene_opacity(state, target), output.min, output.max)
                    }
                    MidiValueSource::SceneInputDimmer { ref target } => {
                        scale_to_range(scene_input_dimmer(state, target), output.min, output.max)
                    }
                    MidiValueSource::SceneBeatOffset { ref target } => {
                        scale_to_range(scene_beat_offset(state, target), output.min, output.max)
                    }
                    MidiValueSource::SceneIgnoreMainDimmer { ref target } => binary_output_value(
                        scene_ignore_main_dimmer(state, target),
                        output.active_value,
                    ),
                    MidiValueSource::SceneSetOffsetOnFlash { ref target } => binary_output_value(
                        scene_set_offset_on_flash(state, target),
                        output.active_value,
                    ),
                    MidiValueSource::SceneActive { ref target } => {
                        binary_output_value(scene_is_active(state, target), output.active_value)
                    }
                    MidiValueSource::SceneFlashed { ref target } => {
                        binary_output_value(scene_is_flashed(state, target), output.active_value)
                    }
                    MidiValueSource::SceneEffectSettingF32 {
                        ref target,
                        effect_index,
                        setting_index,
                    } => scale_to_range(
                        scene_effect_setting_f32(
                            state,
                            target,
                            effect_index as usize,
                            setting_index as usize,
                        ),
                        output.min,
                        output.max,
                    ),
                };
                if selected_test_port && !value_source_uses_active_value(&output.source)
                    && let Some(override_value) = test_state.value_output_overrides.get(&binding_index)
                {
                    midi_value = *override_value;
                }
                if let Err(err) = connection.send(&[output.status, output.data1, midi_value]) {
                    log::error!("Could not send generic midi value output: {err:?}");
                }
                sent_any = true;
            }
            MidiOutputBindingKind::SceneColorValue(output) => {
                let (target, scene_color) = match output.source {
                    MidiColorSource::SceneColor { ref target } => (target, scene_color(state, target)),
                };
                let Some(active_mapping) = mapping
                    .mapping
                    .color_mappings
                    .iter()
                    .find(|named| named.name == output.mapping_name) else {
                    continue;
                };
                let effective_color = scene_color;
                let selected_map = if scene_is_flashed(state, target) {
                    &active_mapping.flashed
                } else if scene_is_active(state, target) {
                    &active_mapping.active
                } else {
                    &active_mapping.inactive
                };
                let message = selected_map.message_for_color(effective_color);
                let midi_value = if selected_test_port {
                    test_state
                        .scene_color_value_overrides
                        .get(&active_mapping.name)
                        .copied()
                        .unwrap_or(message.value)
                } else {
                    message.value
                };
                if let Err(err) =
                    connection.send(&[output.status, output.data1, midi_value])
                {
                    log::error!("Could not send scene color midi value output: {err:?}");
                }
                sent_any = true;
            }
        }
    }
    }

    sent_any
}

fn snapshot_from_project_state(
    state: &ProjectState,
    collections: &Collections,
) -> MidiRuntimeSnapshot {
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
            let mapping = controller.data.mapping.clone();

            mappings_by_controller.insert(
                RuntimeControllerKey::PortName(controller_key.clone()),
                ActiveRuntimeMapping {
                    controller_id: *controller_id,
                    mapping,
                },
            );
            included_controller_ids.insert(*controller_id);
        }
    }

    // Also include controllers that are not mapped in External Devices.
    // They are ignored for normal output routing, but can still be used via Test Device.
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

fn trigger_matches(
    trigger: &crate::storage::asset::midi_controller::MidiTrigger,
    status: u8,
    data1: u8,
) -> bool {
    if trigger.status != status {
        return false;
    }

    if trigger.match_data1 && trigger.data1 != data1 {
        return false;
    }

    true
}

fn resolve_mapping_for_port(
    mappings_by_controller: &HashMap<RuntimeControllerKey, ActiveRuntimeMapping>,
    port_name: &str,
) -> Option<ActiveRuntimeMapping> {
    // Require exact port name match to avoid unintended mappings between different ports
    // of the same multi-port device (e.g., iCON P1-X1 MIDI 1, 2, 3, 4)
    mappings_by_controller.iter().find_map(|(key, mapping)| {
        if matches!(key, RuntimeControllerKey::PortName(name) if name == port_name) {
            Some(mapping.clone())
        } else {
            None
        }
    })
}

fn scale_to_range(value: f32, min: u8, max: u8) -> u8 {
    let min = f32::from(min);
    let max = f32::from(max);
    let value = value.clamp(0.0, 1.0);
    (min + (max - min) * value).round() as u8
}

fn value_source_uses_active_value(source: &MidiValueSource) -> bool {
    matches!(
        source,
        MidiValueSource::SelectedSceneIgnoreMainDimmer
            | MidiValueSource::SelectedSceneSetOffsetOnFlash
            | MidiValueSource::BeatFlankPulse { .. }
            | MidiValueSource::Blackout { .. }
            | MidiValueSource::SceneIgnoreMainDimmer { .. }
            | MidiValueSource::SceneSetOffsetOnFlash { .. }
            | MidiValueSource::SceneActive { .. }
            | MidiValueSource::SceneFlashed { .. }
    )
}

fn binary_output_value(active: bool, active_value: u8) -> u8 {
    if active { active_value } else { 0 }
}

fn beat_range_contains(position: f32, start: f32, end: f32) -> bool {
    let start = start.rem_euclid(4.0);
    let end = end.rem_euclid(4.0);

    if start <= end {
        position >= start && position <= end
    } else {
        position >= start || position <= end
    }
}

fn blackout_output_active(state: &MidiState, inverted: bool, blink: bool) -> bool {
    let base_active = if inverted { !state.blackout } else { state.blackout };
    if !base_active || !blink {
        return base_active;
    }

    let beats_per_second = (state.beats_per_minute / 60.0).max(f32::EPSILON);
    let elapsed_seconds = state.beat_progression / beats_per_second;
    (elapsed_seconds * 2.0).rem_euclid(1.0) < 0.5
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

fn scene_target_to_union(target: &MidiSceneTarget) -> Option<SceneInstanceUnion> {
    match target {
        MidiSceneTarget::Selected => None,
        MidiSceneTarget::Quick { index } => Some(quick_scene_instance_index(*index as usize)),
        MidiSceneTarget::Grid { row, col } => Some(grid_scene_instance_index(
            crate::storage::asset::scene::grid::GridLocation::new(*col as usize, *row as usize),
        )),
    }
}

fn target_location(
    state: &MidiState,
    target: &MidiSceneTarget,
) -> crate::storage::asset::scene::grid::GridLocation {
    match target {
        MidiSceneTarget::Selected => state.selected_scene_location,
        MidiSceneTarget::Quick { index } => {
            if let Some(row) = state.highlighted_row {
                crate::storage::asset::scene::grid::GridLocation::new(*index as usize, row)
            } else if let Some(col) = state.highlighted_col {
                crate::storage::asset::scene::grid::GridLocation::new(col, *index as usize)
            } else {
                crate::storage::asset::scene::grid::GridLocation::new(*index as usize, 0)
            }
        }
        MidiSceneTarget::Grid { row, col } => {
            crate::storage::asset::scene::grid::GridLocation::new(*col as usize, *row as usize)
        }
    }
}

fn scene_color(state: &MidiState, target: &MidiSceneTarget) -> SceneInstanceColor {
    state
        .available_scenes_grid
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(SceneInstanceColor::Black)
}

fn scene_is_active(state: &MidiState, target: &MidiSceneTarget) -> bool {
    state
        .active_scenes
        .contains(&target_location(state, target))
}

fn scene_is_flashed(state: &MidiState, target: &MidiSceneTarget) -> bool {
    state
        .flashed_scenes
        .contains(&target_location(state, target))
}

fn scene_opacity(state: &MidiState, target: &MidiSceneTarget) -> f32 {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_opacity;
    }

    if scene_is_active(state, target) {
        1.0
    } else {
        0.0
    }
}

fn scene_input_dimmer(state: &MidiState, target: &MidiSceneTarget) -> f32 {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_input_dimmer;
    }

    state
        .scene_input_dimmer
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(1.0)
}

fn scene_beat_offset(state: &MidiState, target: &MidiSceneTarget) -> f32 {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_beat_offset;
    }

    state
        .scene_beat_offset
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(0.0)
}

fn scene_ignore_main_dimmer(state: &MidiState, target: &MidiSceneTarget) -> bool {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_ignore_main_dimmer;
    }

    state
        .scene_ignore_main_dimmer
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(false)
}

fn scene_set_offset_on_flash(state: &MidiState, target: &MidiSceneTarget) -> bool {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_set_offset_on_flash;
    }

    state
        .scene_set_offset_on_flash
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(false)
}

fn scene_effect_setting_f32(
    state: &MidiState,
    target: &MidiSceneTarget,
    effect_index: usize,
    setting_index: usize,
) -> f32 {
    state
        .scene_effect_setting_f32
        .get(&(target_location(state, target), effect_index, setting_index))
        .copied()
        .unwrap_or(0.0)
}
