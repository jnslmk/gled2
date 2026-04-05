use crate::storage::asset::midi_controller::{
    MidiControllerMapping, MidiInputAction, MidiInputBinding, MidiNamedSceneColorMapping,
    MidiOutputBinding, MidiOutputBindingKind, MidiValueOutput, MidiValueSource,
};

use super::value_source_converters::{color_source_label, value_source_label};

pub(super) fn input_action_label(action: &MidiInputAction) -> &'static str {
    match action {
        MidiInputAction::Tap => "Tap",
        MidiInputAction::SetMainDimmer => "Set Main Dimmer",
        MidiInputAction::SetBlackout => "Set Blackout",
        MidiInputAction::SetSpeedAdd => "Set Speed Add",
        MidiInputAction::SetSpeedMultiply => "Set Speed Multiply",
        MidiInputAction::SelectScene { .. } => "Select Scene",
        MidiInputAction::ToggleSceneActive { .. } => "Toggle Scene Active",
        MidiInputAction::SetSceneActive { .. } => "Set Scene Active",
        MidiInputAction::SetSceneOpacity { .. } => "Set Scene Opacity",
        MidiInputAction::SetSceneInputDimmer { .. } => "Set Scene Input Dimmer",
        MidiInputAction::SetSceneBeatOffset { .. } => "Set Scene Beat Offset",
        MidiInputAction::SetSceneIgnoreMainDimmer { .. } => "Set Scene Ignore Main Dimmer",
        MidiInputAction::SetSceneSetOffsetOnFlash { .. } => "Set Scene Set Offset On Flash",
        MidiInputAction::SetSceneEffectSettingF32 { .. } => "Set Effect Setting (n,n) f32",
    }
}

pub(super) fn input_binding_matches_filter(binding: &MidiInputBinding, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let f = filter.to_lowercase();
    binding.name.to_lowercase().contains(&f)
        || binding.trigger.status.to_string().contains(filter)
        || binding.trigger.data1.to_string().contains(filter)
        || input_action_label(&binding.action)
            .to_lowercase()
            .contains(&f)
}

pub(super) fn output_binding_matches_filter(binding: &MidiOutputBinding, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let f = filter.to_lowercase();
    if binding.name.to_lowercase().contains(&f) {
        return true;
    }
    match &binding.kind {
        MidiOutputBindingKind::Value(v) => {
            v.status.to_string().contains(filter)
                || v.data1.to_string().contains(filter)
                || value_source_label(&v.source).to_lowercase().contains(&f)
        }
        MidiOutputBindingKind::SceneColorValue(v) => {
            v.status.to_string().contains(filter)
                || v.data1.to_string().contains(filter)
                || color_source_label(&v.source).to_lowercase().contains(&f)
                || "scene color value".contains(&f)
        }
    }
}

fn output_source_from_input_action(action: &MidiInputAction) -> Option<MidiValueSource> {
    match action {
        MidiInputAction::SetMainDimmer => Some(MidiValueSource::MainDimmer),
        MidiInputAction::SetBlackout => Some(MidiValueSource::Blackout {
            inverted: false,
            blink: false,
        }),
        MidiInputAction::SetSceneActive { target } => Some(MidiValueSource::SceneActive {
            target: target.clone(),
        }),
        MidiInputAction::SetSceneOpacity { target } => Some(match target {
            crate::storage::asset::midi_controller::MidiSceneTarget::Selected => {
                MidiValueSource::SelectedSceneOpacity
            }
            _ => MidiValueSource::SceneOpacity {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneInputDimmer { target } => Some(match target {
            crate::storage::asset::midi_controller::MidiSceneTarget::Selected => {
                MidiValueSource::SelectedSceneInputDimmer
            }
            _ => MidiValueSource::SceneInputDimmer {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneBeatOffset { target } => Some(match target {
            crate::storage::asset::midi_controller::MidiSceneTarget::Selected => {
                MidiValueSource::SelectedSceneBeatOffset
            }
            _ => MidiValueSource::SceneBeatOffset {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneIgnoreMainDimmer { target } => Some(match target {
            crate::storage::asset::midi_controller::MidiSceneTarget::Selected => {
                MidiValueSource::SelectedSceneIgnoreMainDimmer
            }
            _ => MidiValueSource::SceneIgnoreMainDimmer {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneSetOffsetOnFlash { target } => Some(match target {
            crate::storage::asset::midi_controller::MidiSceneTarget::Selected => {
                MidiValueSource::SelectedSceneSetOffsetOnFlash
            }
            _ => MidiValueSource::SceneSetOffsetOnFlash {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneEffectSettingF32 {
            target,
            effect_index,
            setting_index,
        } => Some(MidiValueSource::SceneEffectSettingF32 {
            target: target.clone(),
            effect_index: *effect_index,
            setting_index: *setting_index,
        }),
        MidiInputAction::Tap
        | MidiInputAction::SetSpeedAdd
        | MidiInputAction::SetSpeedMultiply
        | MidiInputAction::SelectScene { .. }
        | MidiInputAction::ToggleSceneActive { .. } => None,
    }
}

pub(super) fn add_matching_output_binding(
    mapping: &mut MidiControllerMapping,
    input_name: &str,
    input_status: u8,
    input_data1: u8,
    action: &MidiInputAction,
) -> bool {
    let source = output_source_from_input_action(action).unwrap_or(MidiValueSource::SelectedSceneOpacity);

    let already_exists = mapping.output_bindings.iter().any(|binding| {
        matches!(
            &binding.kind,
            MidiOutputBindingKind::Value(value)
                if value.status == input_status
                    && value.data1 == input_data1
                    && value.source == source
        )
    });

    if already_exists {
        return false;
    }

    mapping.output_bindings.push(MidiOutputBinding {
        name: if input_name.is_empty() {
            "Output".to_owned()
        } else {
            format!("{} Output", input_name)
        },
        kind: MidiOutputBindingKind::Value(MidiValueOutput {
            status: input_status,
            data1: input_data1,
            min: 0,
            max: 127,
            active_value: 127,
            source,
        }),
    });

    true
}

pub(super) fn output_binding_status_data1(
    binding: &MidiOutputBinding,
    _color_mappings: &[MidiNamedSceneColorMapping],
) -> Option<(u8, u8)> {
    match &binding.kind {
        MidiOutputBindingKind::Value(value) => Some((value.status, value.data1)),
        MidiOutputBindingKind::SceneColorValue(scene_color) => {
            Some((scene_color.status, scene_color.data1))
        }
    }
}

pub(super) fn set_output_binding_status_data1(
    binding: &mut MidiOutputBinding,
    _color_mappings: &mut [MidiNamedSceneColorMapping],
    status: u8,
    data1: u8,
) -> bool {
    match &mut binding.kind {
        MidiOutputBindingKind::Value(value) => {
            let changed = value.status != status || value.data1 != data1;
            value.status = status;
            value.data1 = data1;
            changed
        }
        MidiOutputBindingKind::SceneColorValue(scene_color) => {
            let changed = scene_color.status != status || scene_color.data1 != data1;
            scene_color.status = status;
            scene_color.data1 = data1;
            changed
        }
    }
}

pub(super) fn midi_status_label(status: u8) -> String {
    if status >= 0xF0 {
        return format!("System (0x{status:02X})");
    }

    let channel = (status & 0x0F) + 1;
    let kind = match status & 0xF0 {
        0x80 => "Note Off",
        0x90 => "Note On",
        0xA0 => "Poly Aftertouch",
        0xB0 => "Control Change",
        0xC0 => "Program Change",
        0xD0 => "Channel Pressure",
        0xE0 => "Pitch Bend",
        _ => "Unknown",
    };

    format!("{kind} (ch {channel}, 0x{status:02X})")
}
