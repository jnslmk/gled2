use crate::{
    midi::state::MidiState,
    midi::normalize_controller_key,
    output_state::ProjectState,
    storage::{
        asset::{
            Asset,
            midi_controller::{
                MidiColorSource, MidiController, MidiControllerMapping, MidiInputAction,
                MidiOutputBindingKind, MidiSceneTarget, MidiValueSource,
            },
            project::Project,
            project::scene_instance_path::{SceneInstanceUnion, grid_scene_instance_index, quick_scene_instance_index},
            scene::color::SceneInstanceColor,
        },
        collections::Collections,
    },
    ui::action::UiAction,
};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, unbounded};
use midir::MidiOutputConnection;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::spawn;

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
    mappings_by_controller: HashMap<String, ActiveRuntimeMapping>,
}

#[derive(Clone)]
struct ActiveRuntimeMapping {
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
            MidiInputAction::SetSpeedMultiply => Some(UiAction::SpeedMultiply(if value > 63 {
                2.0
            } else {
                0.5
            })),
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
                MidiSceneTarget::Selected => {
                    Some(UiAction::SetSelectedSceneInputDimmer(f32::from(value) / 127.0))
                }
                _ => scene_target_to_union(target).map(|target| {
                    UiAction::SetSceneInputDimmer(target, f32::from(value) / 127.0)
                }),
            },
            MidiInputAction::SetSceneBeatOffset { ref target } => match target {
                MidiSceneTarget::Selected => {
                    Some(UiAction::SetSelectedSceneBeatOffset(f32::from(value) / 127.0))
                }
                _ => scene_target_to_union(target)
                    .map(|target| UiAction::SetSceneBeatOffset(target, f32::from(value) / 127.0)),
            },
            MidiInputAction::SetSceneIgnoreMainDimmer { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneIgnoreMainDimmer(
                    value > 63,
                )),
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
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneEffectAnimationConfigF32(
                    effect_index as usize,
                    setting_index as usize,
                    f32::from(value) / 127.0,
                )),
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
            return true;
        }
    }

    false
}

fn send_output_from_snapshot(
    snapshot: &MidiRuntimeSnapshot,
    port_name: &str,
    state: &MidiState,
    connection: &mut MidiOutputConnection,
) -> bool {
    let mapping = resolve_mapping_for_port(&snapshot.mappings_by_controller, port_name);
    let Some(mapping) = mapping else {
        return false;
    };

    for binding in &mapping.mapping.output_bindings {
        match &binding.kind {
            MidiOutputBindingKind::Value(output) => {
                let value = match output.source {
                    MidiValueSource::SelectedSceneOpacity => state.selected_scene_opacity,
                    MidiValueSource::SelectedSceneInputDimmer => state.selected_scene_input_dimmer,
                    MidiValueSource::SelectedSceneBeatOffset => state.selected_scene_beat_offset,
                    MidiValueSource::SelectedSceneIgnoreMainDimmer => {
                        if state.selected_scene_ignore_main_dimmer {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    MidiValueSource::SelectedSceneSetOffsetOnFlash => {
                        if state.selected_scene_set_offset_on_flash {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    MidiValueSource::MainDimmer => state.main_dimmer,
                    MidiValueSource::BeatFlank => f32::from(state.beat_flank.min(4)) / 4.0,
                    MidiValueSource::Blackout => {
                        if state.blackout { 1.0 } else { 0.0 }
                    }
                    MidiValueSource::SceneOpacity { ref target } => scene_opacity(state, target),
                    MidiValueSource::SceneInputDimmer { ref target } => {
                        scene_input_dimmer(state, target)
                    }
                    MidiValueSource::SceneBeatOffset { ref target } => {
                        scene_beat_offset(state, target)
                    }
                    MidiValueSource::SceneIgnoreMainDimmer { ref target } => {
                        if scene_ignore_main_dimmer(state, target) {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    MidiValueSource::SceneSetOffsetOnFlash { ref target } => {
                        if scene_set_offset_on_flash(state, target) {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    MidiValueSource::SceneActive { ref target } => {
                        if scene_is_active(state, target) { 1.0 } else { 0.0 }
                    }
                    MidiValueSource::SceneFlashed { ref target } => {
                        if scene_is_flashed(state, target) { 1.0 } else { 0.0 }
                    }
                    MidiValueSource::SceneEffectSettingF32 {
                        ref target,
                        effect_index,
                        setting_index,
                    } => scene_effect_setting_f32(
                        state,
                        target,
                        effect_index as usize,
                        setting_index as usize,
                    ),
                };
                let midi_value = scale_to_range(value, output.min, output.max);
                if let Err(err) = connection.send(&[output.status, output.data1, midi_value]) {
                    log::error!("Could not send generic midi value output: {err:?}");
                }
            }
            MidiOutputBindingKind::ColorChannels(output) => {
                let color = match output.source {
                    MidiColorSource::SelectedSceneColor => state.selected_scene_color,
                    MidiColorSource::SceneColor { ref target } => scene_color(state, target),
                };
                let midi_value = scene_color_to_midi_value(color);
                if let Err(err) = connection.send(&[output.status, output.data1, midi_value]) {
                    log::error!("Could not send generic midi color output: {err:?}");
                }
            }
        }
    }

    true
}

fn snapshot_from_project_state(state: &ProjectState, collections: &Collections) -> MidiRuntimeSnapshot {
    let mut mappings_by_controller = HashMap::new();

    let Some(project) = state.project.as_ref() else {
        return MidiRuntimeSnapshot {
            mappings_by_controller,
        };
    };

    for (controller_key, selection) in &project.midi_active_mappings {
        let Some(controller_id) = selection else {
            continue;
        };
        let Some(controller) = Asset::<MidiController>::get(*controller_id, collections) else {
            continue;
        };
        let mapping = controller.data.mapping.clone();

        // Only load mappings with exact port names (not normalized keys from old versions)
        // Exact port names contain port identifiers like "MIDI 1", "MIDI 2", etc.
        // If the key equals its normalized form, it's likely an old normalized entry - skip it
        let normalized = normalize_controller_key(controller_key);
        if controller_key == &normalized {
            // This looks like an already-normalized key - skip it as it's from old data
            log::debug!(
                "Skipping old normalized-key mapping: {}",
                controller_key
            );
            continue;
        }

        mappings_by_controller.insert(controller_key.clone(), ActiveRuntimeMapping { mapping });
    }

    MidiRuntimeSnapshot {
        mappings_by_controller,
    }
}

fn trigger_matches(trigger: &crate::storage::asset::midi_controller::MidiTrigger, status: u8, data1: u8) -> bool {
    if trigger.status != status {
        return false;
    }

    if trigger.match_data1 && trigger.data1 != data1 {
        return false;
    }

    true
}

fn resolve_mapping_for_port(
    mappings_by_controller: &HashMap<String, ActiveRuntimeMapping>,
    port_name: &str,
) -> Option<ActiveRuntimeMapping> {
    // Require exact port name match to avoid unintended mappings between different ports
    // of the same multi-port device (e.g., iCON P1-X1 MIDI 1, 2, 3, 4)
    mappings_by_controller.get(port_name).cloned()
}

fn scale_to_range(value: f32, min: u8, max: u8) -> u8 {
    let min = f32::from(min);
    let max = f32::from(max);
    let value = value.clamp(0.0, 1.0);
    (min + (max - min) * value).round() as u8
}

fn scene_color_to_midi_value(color: SceneInstanceColor) -> u8 {
    match color {
        SceneInstanceColor::Black => 0,
        SceneInstanceColor::White => 3,
        SceneInstanceColor::Red => 5,
        SceneInstanceColor::Orange => 9,
        SceneInstanceColor::Yellow => 13,
        SceneInstanceColor::Green => 21,
        SceneInstanceColor::Blue => 45,
        SceneInstanceColor::Purple => 49,
        SceneInstanceColor::Pink => 53,
    }
}

pub fn selected_scene_color(project: Option<&mut Project>, fallback: SceneInstanceColor, selected: crate::storage::asset::scene::grid::GridLocation) -> SceneInstanceColor {
    project
        .and_then(|project| project.get_scenes_instance(&selected).map(|scene| scene.color))
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

fn target_location(state: &MidiState, target: &MidiSceneTarget) -> crate::storage::asset::scene::grid::GridLocation {
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
    state.active_scenes.contains(&target_location(state, target))
}

fn scene_is_flashed(state: &MidiState, target: &MidiSceneTarget) -> bool {
    state.flashed_scenes.contains(&target_location(state, target))
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
