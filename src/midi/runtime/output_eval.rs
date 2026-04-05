use crate::{
    midi::normalize_controller_key,
    midi::state::MidiState,
    storage::asset::midi_controller::{MidiOutputBindingKind, MidiValueSource},
};
use midir::MidiOutputConnection;

use super::{
    ActiveRuntimeMapping, RuntimeControllerKey, scene_state,
    runtime_test_execution_state,
};

pub(super) fn send_output_from_snapshot(
    mappings_by_controller: &std::collections::HashMap<RuntimeControllerKey, ActiveRuntimeMapping>,
    port_name: &str,
    state: &MidiState,
    connection: &mut MidiOutputConnection,
) -> bool {
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
                        } => {
                            binary_output_value(
                                beat_range_contains(
                                    state.beat_progression.rem_euclid(4.0),
                                    start_beat,
                                    end_beat,
                                ),
                                output.active_value,
                            )
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
                    if let Err(err) = connection.send(&[output.status, output.data1, midi_value]) {
                        log::error!("Could not send generic midi value output: {err:?}");
                    }
                    sent_any = true;
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
                    if let Err(err) = connection.send(&[output.status, output.data1, message.value]) {
                        log::error!("Could not send scene color midi value output: {err:?}");
                    }
                    sent_any = true;
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
                    if let Err(err) = connection.send(&[*status, *data1, *value]) {
                        log::error!("Could not send test device output: {err:?}");
                    } else {
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
