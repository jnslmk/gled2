use crate::{
    midi::learn::{LearnState, MidiLearnRequest, MidiLearnTarget},
    storage::asset::midi_controller::{MidiController, MidiInputAction, MidiInputBinding},
};
use egui::{ComboBox, DragValue, Ui};
use egui_phosphor_icons::icons;

use super::{
    action_converters::{
        input_action_from_kind, input_action_kind, scene_target_editor, MidiInputActionKind,
    },
    binding_helpers::{input_binding_matches_filter, midi_status_label},
    iconized,
};

pub(super) fn render_input_column(
    ui: &mut Ui,
    bindings: &mut Vec<MidiInputBinding>,
    filter: &str,
    hovered_status_data1: Option<(u8, u8)>,
    hovered_status_data1_next: &mut Option<(u8, u8)>,
    dirty: &mut bool,
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    learn_state: &mut LearnState,
) -> (Option<usize>, Vec<(String, u8, u8, MidiInputAction)>) {
    let mut remove_input = None;
    let mut pending_matching_output = Vec::new();
    let mut duplicate_inputs = Vec::new();

    ui.horizontal(|ui| {
        ui.label(icons::ARROW_SQUARE_IN);
        ui.heading("Input Bindings");
    });
    ui.horizontal(|ui| {
        if ui
            .button(iconized(ui, icons::PLUS, " Add Input Binding"))
            .clicked()
        {
            bindings.push(MidiInputBinding::default());
            *dirty = true;
        }
    });

    for (index, binding) in bindings.iter_mut().enumerate() {
        if !input_binding_matches_filter(binding, filter) {
            continue;
        }
        let status_data1 = (binding.trigger.status, binding.trigger.data1);
        let highlight = hovered_status_data1.is_some_and(|hovered| hovered == status_data1);
        let base_stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        let response = egui::Frame::group(ui.style())
            .stroke(if highlight {
                egui::Stroke::new(base_stroke.width, ui.visuals().selection.stroke.color)
            } else {
                base_stroke
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Input {}", index + 1));
                    let learning = learn_state.active_request().is_some_and(|request| {
                        request.controller_id == controller_id
                            && request.target == MidiLearnTarget::InputBinding(index)
                    });
                    if ui
                        .button(if learning {
                            iconized(ui, icons::MICROPHONE, " Waiting for MIDI...")
                        } else {
                            iconized(ui, icons::MICROPHONE, " Learn")
                        })
                        .clicked()
                    {
                        learn_state.arm(MidiLearnRequest {
                            controller_id,
                            target: MidiLearnTarget::InputBinding(index),
                        });
                    }
                    if ui
                        .button(iconized(ui, icons::PLUS, " Duplicate"))
                        .clicked()
                    {
                        duplicate_inputs.push(binding.clone());
                    }
                    if ui.button(iconized(ui, icons::TRASH, " Remove")).clicked() {
                        remove_input = Some(index);
                    }
                });

                if ui.text_edit_singleline(&mut binding.name).changed() {
                    *dirty = true;
                }

                ui.horizontal(|ui| {
                    ui.label("Status");
                    if ui
                        .add(DragValue::new(&mut binding.trigger.status).range(0..=255))
                        .changed()
                    {
                        *dirty = true;
                    }
                    ui.label("Data1");
                    if ui
                        .add(DragValue::new(&mut binding.trigger.data1).range(0..=127))
                        .changed()
                    {
                        *dirty = true;
                    }
                });

                ui.label(format!(
                    "Detected Message: {}",
                    midi_status_label(binding.trigger.status)
                ));

                if ui
                    .checkbox(
                        &mut binding.trigger.match_data1,
                        "Match Data1 (disable for Pitch Bend style controls)",
                    )
                    .changed()
                {
                    *dirty = true;
                }

                let mut action_kind = input_action_kind(&binding.action);
                ComboBox::new(format!("{}_input_kind_{index}", controller_id), "")
                    .selected_text(format!("{action_kind:?}"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut action_kind, MidiInputActionKind::Tap, "Tap");
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetMainDimmer,
                            "Set Main Dimmer",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetBlackout,
                            "Set Blackout",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetSpeedAdd,
                            "Set Speed Add",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetSpeedMultiply,
                            "Set Speed Multiply",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SelectScene,
                            "Select Scene",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::ToggleSceneActive,
                            "Toggle Scene Active",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetSceneActive,
                            "Set Scene Active",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetSceneOpacity,
                            "Set Scene Opacity",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetSceneInputDimmer,
                            "Set Scene Input Dimmer",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetSceneBeatOffset,
                            "Set Scene Beat Offset",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetSceneIgnoreMainDimmer,
                            "Set Scene Ignore Main Dimmer",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetSceneSetOffsetOnFlash,
                            "Set Scene Set Offset On Flash",
                        );
                        ui.selectable_value(
                            &mut action_kind,
                            MidiInputActionKind::SetSceneEffectSettingF32,
                            "Set Effect Setting (n,n) f32",
                        );
                    });

                let before = binding.action.clone();
                binding.action = input_action_from_kind(action_kind, binding.action.clone());
                if binding.action != before {
                    *dirty = true;
                }

                match &mut binding.action {
                    crate::storage::asset::midi_controller::MidiInputAction::SelectScene { target }
                    | crate::storage::asset::midi_controller::MidiInputAction::ToggleSceneActive {
                        target
                    }
                    | crate::storage::asset::midi_controller::MidiInputAction::SetSceneActive { target }
                    | crate::storage::asset::midi_controller::MidiInputAction::SetSceneOpacity { target }
                    | crate::storage::asset::midi_controller::MidiInputAction::SetSceneInputDimmer {
                        target
                    }
                    | crate::storage::asset::midi_controller::MidiInputAction::SetSceneBeatOffset {
                        target
                    }
                    | crate::storage::asset::midi_controller::MidiInputAction::SetSceneIgnoreMainDimmer {
                        target
                    }
                    | crate::storage::asset::midi_controller::MidiInputAction::SetSceneSetOffsetOnFlash {
                        target
                    }
                    | crate::storage::asset::midi_controller::MidiInputAction::SetSceneEffectSettingF32 {
                        target,
                        ..
                    } => {
                        scene_target_editor(
                            ui,
                            target,
                            dirty,
                            format!("{}_input_target_{index}", controller_id),
                        );
                    }
                    _ => {}
                }

                if let crate::storage::asset::midi_controller::MidiInputAction::SetSceneEffectSettingF32 {
                    effect_index,
                    setting_index,
                    ..
                } = &mut binding.action
                {
                    ui.horizontal(|ui| {
                        ui.label("Effect Index");
                        if ui.add(DragValue::new(effect_index).range(0..=255)).changed() {
                            *dirty = true;
                        }
                        ui.label("Setting Index");
                        if ui.add(DragValue::new(setting_index).range(0..=255)).changed() {
                            *dirty = true;
                        }
                    });
                }

                if ui
                    .button(iconized(ui, icons::PLUS, " Create Matching Output Binding"))
                    .clicked()
                {
                    pending_matching_output.push((
                        binding.name.clone(),
                        binding.trigger.status,
                        binding.trigger.data1,
                        binding.action.clone(),
                    ));
                }
            })
            .response;
        if response.hovered() {
            *hovered_status_data1_next = Some(status_data1);
        }
    }

    if !duplicate_inputs.is_empty() {
        bindings.extend(duplicate_inputs);
        *dirty = true;
    }

    (remove_input, pending_matching_output)
}
