use crate::{
    midi::normalize_controller_key,
    midi::state::MidiState,
    storage::asset::midi_controller::{MidiOutputBindingKind, MidiValueSource},
};
use midir::MidiOutputConnection;

use super::{
    MidiRuntimeSnapshot, RuntimeControllerKey, scene_state,
    runtime_test_execution_state,
};

pub(super) fn send_output_from_snapshot(
    snapshot: &MidiRuntimeSnapshot,
    port_name: &str,
    state: &MidiState,
    connection: &mut MidiOutputConnection,
    last_sent_outputs: &mut std::collections::HashMap<(u8, u8), u8>,
    last_input_values: &std::collections::HashMap<(u8, u8), u8>,
) -> bool {
    let mappings_by_controller = &snapshot.mappings_by_controller;
    let normalized_port_name = normalize_controller_key(port_name);
    let mut sent_any = false;

    // Send normal routed outputs
    for (controller_key, mapping) in mappings_by_controller {
        let routed_to_port = matches!(
            controller_key,
            RuntimeControllerKey::PortName(name)
                if name == port_name || normalize_controller_key(name) == normalized_port_name
        );
        if !routed_to_port {
            continue;
        }

        let effective_mapping = &mapping.mapping;

            for binding in &effective_mapping.output_bindings {
            match &binding.kind {
                MidiOutputBindingKind::Value(output) => {
                    let midi_value = match output.source {
                        MidiValueSource::SelectedSceneOpacity => {
                            scale_to_range(state.selected_scene_opacity, output.min, output.max)
                        }
                        MidiValueSource::SelectedSceneDistributed => {
                            let value = last_input_values
                                .get(&(output.status, output.data1))
                                .copied()
                                .unwrap_or_else(|| {
                                    let scene_count = snapshot.scene_locations_row_major.len();
                                    let selected_index = snapshot
                                        .scene_locations_row_major
                                        .iter()
                                        .position(|location| *location == state.selected_scene_location)
                                        .unwrap_or(0);

                                    value_for_distributed_scene_index(selected_index, scene_count)
                                });
                            scale_to_range(f32::from(value) / 127.0, output.min, output.max)
                        }
                        MidiValueSource::SelectedSceneInputDimmer => {
                            scale_to_range(state.selected_scene_input_dimmer, output.min, output.max)
                        }
                        MidiValueSource::SelectedSceneBeatOffset => {
                            scale_to_range(state.selected_scene_beat_offset, output.min, output.max)
                        }
                        MidiValueSource::SelectedSceneIgnoreMainDimmer => {
                            binary_output_value(state.selected_scene_ignore_main_dimmer, output.active_value)
                        }
                        MidiValueSource::SelectedSceneSetOffsetOnFlash => {
                            binary_output_value(state.selected_scene_set_offset_on_flash, output.active_value)
                        }
                        MidiValueSource::MainDimmer => {
                            scale_to_range(state.main_dimmer, output.min, output.max)
                        }
                        MidiValueSource::BeatFlankPulse {
                            start_beat,
                            end_beat,
                            blackout_blink_value,
                        } => {
                            if state.blackout {
                                if let Some(blink_value) = blackout_blink_value {
                                    let bps = (state.beats_per_minute / 60.0).max(f32::EPSILON);
                                    let elapsed = state.beat_progression / bps;
                                    if (elapsed * 4.0).rem_euclid(1.0) < 0.5 {
                                        blink_value
                                    } else {
                                        0
                                    }
                                } else {
                                    0
                                }
                            } else {
                                binary_output_value(
                                    beat_range_contains(
                                        state.beat_progression.rem_euclid(4.0),
                                        start_beat,
                                        end_beat,
                                    ),
                                    output.active_value,
                                )
                            }
                        }
                        MidiValueSource::Blackout { inverted, blink } => {
                            binary_output_value(blackout_output_active(state, inverted, blink), output.active_value)
                        }
                        MidiValueSource::SceneOpacity { ref target } => {
                            scale_to_range(scene_state::scene_opacity(state, target), output.min, output.max)
                        }
                        MidiValueSource::SceneInputDimmer { ref target } => {
                            scale_to_range(scene_state::scene_input_dimmer(state, target), output.min, output.max)
                        }
                        MidiValueSource::SceneBeatOffset { ref target } => {
                            scale_to_range(scene_state::scene_beat_offset(state, target), output.min, output.max)
                        }
                        MidiValueSource::SceneIgnoreMainDimmer { ref target } => {
                            binary_output_value(scene_state::scene_ignore_main_dimmer(state, target), output.active_value)
                        }
                        MidiValueSource::SceneSetOffsetOnFlash { ref target } => {
                            binary_output_value(scene_state::scene_set_offset_on_flash(state, target), output.active_value)
                        }
                        MidiValueSource::SceneActive { ref target } => {
                            binary_output_value(scene_state::scene_is_active(state, target), output.active_value)
                        }
                        MidiValueSource::SceneFlashed { ref target } => {
                            binary_output_value(scene_state::scene_is_flashed(state, target), output.active_value)
                        }
                        MidiValueSource::SceneEffectSettingF32 {
                            ref target,
                            effect_index,
                            setting_index,
                        } => scale_to_range(
                            scene_state::scene_effect_setting_f32(
                                state,
                                target,
                                effect_index as usize,
                                setting_index as usize,
                            ),
                            output.min,
                            output.max,
                        ),
                    };
                    if send_if_changed(
                        connection,
                        last_sent_outputs,
                        output.status,
                        output.data1,
                        midi_value,
                        "generic midi value output",
                    ) {
                        sent_any = true;
                    }
                }
                MidiOutputBindingKind::SceneColorValue(output) => {
                    let (target, scene_color) = match output.source {
                        crate::storage::asset::midi_controller::MidiColorSource::SceneColor {
                            ref target,
                        } => (target, scene_state::scene_color(state, target)),
                    };
                    let Some(active_mapping) = effective_mapping
                        .color_mappings
                        .iter()
                        .find(|named| named.name == output.mapping_name)
                    else {
                        continue;
                    };
                    let selected_map = if scene_state::scene_is_flashed(state, target) {
                        &active_mapping.flashed
                    } else if scene_state::scene_is_active(state, target) {
                        &active_mapping.active
                    } else {
                        &active_mapping.inactive
                    };
                    let message = selected_map.message_for_color(scene_color);
                    if send_if_changed(
                        connection,
                        last_sent_outputs,
                        output.status,
                        output.data1,
                        message.value,
                        "scene color midi value output",
                    ) {
                        sent_any = true;
                    }
                }
            }
        }
    }

    // Send active test outputs if test device is selected for this port
    {
        let exec_state = runtime_test_execution_state()
            .lock()
            .expect("test execution state lock poisoned");
        
        if let Some(test_port) = &exec_state.selected_test_device {
            let test_port_normalized = normalize_controller_key(test_port);
            let current_port_normalized = normalize_controller_key(port_name);
            let is_test_port = test_port == port_name
                || test_port_normalized == current_port_normalized;
            
            if is_test_port {
                for ((status, data1), value) in &exec_state.active_outputs {
                    if send_if_changed(
                        connection,
                        last_sent_outputs,
                        *status,
                        *data1,
                        *value,
                        "test device output",
                    ) {
                        sent_any = true;
                    }
                }
            }
        }
    }

    sent_any
}

fn scale_to_range(value: f32, min: u8, max: u8) -> u8 {
    let min = f32::from(min);
    let max = f32::from(max);
    let value = value.clamp(0.0, 1.0);
    (min + (max - min) * value).round() as u8
}

fn send_if_changed(
    connection: &mut MidiOutputConnection,
    last_sent_outputs: &mut std::collections::HashMap<(u8, u8), u8>,
    status: u8,
    data1: u8,
    value: u8,
    label: &str,
) -> bool {
    let key = (status, data1);
    if last_sent_outputs.get(&key).copied() == Some(value) {
        return false;
    }

    if let Err(err) = connection.send(&[status, data1, value]) {
        log::error!("Could not send {label}: {err:?}");
        return false;
    }

    last_sent_outputs.insert(key, value);
    true
}

fn binary_output_value(active: bool, active_value: u8) -> u8 {
    if active {
        active_value
    } else {
        0
    }
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

fn distributed_scene_index_from_midi(value: u8, scene_count: usize) -> Option<usize> {
    if scene_count == 0 {
        return None;
    }

    Some((((usize::from(value) + 1) * scene_count).saturating_sub(1)) / 128)
}

fn value_for_distributed_scene_index(index: usize, scene_count: usize) -> u8 {
    if scene_count == 0 {
        return 0;
    }

    for value in 0u8..=127u8 {
        if distributed_scene_index_from_midi(value, scene_count) == Some(index) {
            let mut last = value;
            for candidate in value..=127u8 {
                if distributed_scene_index_from_midi(candidate, scene_count) == Some(index) {
                    last = candidate;
                } else {
                    break;
                }
            }
            return ((u16::from(value) + u16::from(last)) / 2) as u8;
        }
    }

    // More scenes than values can make some indices unreachable; use monotonic fallback.
    if scene_count <= 1 {
        return 0;
    }
    ((index.min(scene_count - 1) * 127) / (scene_count - 1)) as u8
}

#[cfg(test)]
mod tests {
    use super::{distributed_scene_index_from_midi, value_for_distributed_scene_index};

    #[test]
    fn distributed_index_covers_first_and_last_scene() {
        assert_eq!(distributed_scene_index_from_midi(0, 5), Some(0));
        assert_eq!(distributed_scene_index_from_midi(127, 5), Some(4));
    }

    #[test]
    fn distributed_value_round_trips_for_reachable_indices() {
        let scene_count = 24;
        for index in 0..scene_count {
            let value = value_for_distributed_scene_index(index, scene_count);
            assert_eq!(distributed_scene_index_from_midi(value, scene_count), Some(index));
        }
    }
}
