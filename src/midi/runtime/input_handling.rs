use crate::{
    midi::normalize_controller_key,
    storage::asset::midi_controller::{MidiInputAction, MidiSceneTarget},
    ui::action::UiAction,
};
use std::collections::HashMap;

use super::{
    ActiveRuntimeMapping, MidiRuntimeSnapshot, RuntimeControllerKey,
    scene_state::scene_target_to_union,
};

pub(super) fn handle_input_from_snapshot(
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
            MidiInputAction::SetMainDimmer => Some(UiAction::SetMainDimmer(f32::from(value) / 127.0)),
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
                _ => scene_target_to_union(target).map(|target| UiAction::SetSceneActive(target, value > 0)),
            },
            MidiInputAction::SetSceneOpacity { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneOpacity(f32::from(value) / 127.0)),
                _ => scene_target_to_union(target).map(|target| UiAction::SetSceneOpacity(target, f32::from(value) / 127.0)),
            },
            MidiInputAction::SetSceneInputDimmer { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneInputDimmer(f32::from(value) / 127.0)),
                _ => scene_target_to_union(target).map(|target| UiAction::SetSceneInputDimmer(target, f32::from(value) / 127.0)),
            },
            MidiInputAction::SetSceneBeatOffset { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneBeatOffset(f32::from(value) / 127.0)),
                _ => scene_target_to_union(target).map(|target| UiAction::SetSceneBeatOffset(target, f32::from(value) / 127.0)),
            },
            MidiInputAction::SetSceneIgnoreMainDimmer { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneIgnoreMainDimmer(value > 63)),
                _ => scene_target_to_union(target).map(|target| UiAction::SetSceneIgnoreMainDimmer(target, value > 63)),
            },
            MidiInputAction::SetSceneSetOffsetOnFlash { ref target } => match target {
                MidiSceneTarget::Selected => Some(UiAction::SetSelectedSceneSetOffsetOnFlash(value > 63)),
                _ => scene_target_to_union(target).map(|target| UiAction::SetSceneSetOffsetOnFlash(target, value > 63)),
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
            handled_any = true;
        }
    }

    handled_any
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
    // Prefer exact match, then fall back to normalized match (e.g. ALSA names with numeric suffixes).
    let normalized_port_name = normalize_controller_key(port_name);
    mappings_by_controller.iter().find_map(|(key, mapping)| {
        if matches!(
            key,
            RuntimeControllerKey::PortName(name)
                if name == port_name || normalize_controller_key(name) == normalized_port_name
        ) {
            Some(mapping.clone())
        } else {
            None
        }
    })
}
