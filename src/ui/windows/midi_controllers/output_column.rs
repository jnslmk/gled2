use crate::{
    midi::learn::{LearnState, MidiLearnRequest, MidiLearnTarget},
    storage::asset::midi_controller::{
        MidiController, MidiNamedSceneColorMapping, MidiOutputBinding, MidiOutputBindingKind,
        MidiValueOutput,
    },
};
use egui::{ComboBox, DragValue, Ui};
use egui_phosphor_icons::icons;
use std::collections::HashSet;

use super::{
    binding_helpers::{
        output_binding_matches_filter, output_binding_status_data1, set_output_binding_status_data1,
    },
    color_editor::scene_color_value_output_editor,
    iconized,
    value_editor::value_output_editor,
};

pub(super) fn render_output_column(
    ui: &mut Ui,
    bindings: &mut Vec<MidiOutputBinding>,
    color_mappings: &mut [MidiNamedSceneColorMapping],
    test_device_selected: bool,
    filter: &str,
    hovered_status_data1: Option<(u8, u8)>,
    hovered_status_data1_next: &mut Option<(u8, u8)>,
    value_preview_binding_indices: &mut HashSet<usize>,
    blackout_blink_preview_binding_indices: &mut HashSet<usize>,
    dirty: &mut bool,
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    learn_state: &mut LearnState,
) -> Option<usize> {
    let mut remove_output = None;
    let mut duplicate_outputs = Vec::new();

    ui.horizontal(|ui| {
        ui.label(icons::ARROW_SQUARE_OUT);
        ui.heading("Output Bindings");
    });
    if ui
        .button(iconized(ui, icons::PLUS, " Add Output Binding"))
        .clicked()
    {
        bindings.insert(0, MidiOutputBinding::default());
        *dirty = true;
    }

    for (index, binding) in bindings.iter_mut().enumerate() {
        if !output_binding_matches_filter(binding, filter) {
            continue;
        }
        let mut value_binding_preview_active = false;
        let status_data1 = output_binding_status_data1(binding, color_mappings);
        let highlight = hovered_status_data1
            .zip(status_data1)
            .is_some_and(|(hovered, output)| hovered == output);
        let base_stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        let response = egui::Frame::group(ui.style())
            .stroke(if highlight {
                egui::Stroke::new(base_stroke.width, ui.visuals().selection.stroke.color)
            } else {
                base_stroke
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Output {}", index + 1));
                    let learning = learn_state.active_request().is_some_and(|request| {
                        request.controller_id == controller_id
                            && request.target == MidiLearnTarget::OutputBinding(index)
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
                            target: MidiLearnTarget::OutputBinding(index),
                        });
                    }
                    if ui
                        .button(iconized(ui, icons::PLUS, " Duplicate"))
                        .clicked()
                    {
                        duplicate_outputs.push(binding.clone());
                    }
                    if ui.button(iconized(ui, icons::TRASH, " Remove")).clicked() {
                        remove_output = Some(index);
                    }
                });

                if ui.text_edit_singleline(&mut binding.name).changed() {
                    *dirty = true;
                }

                let mut status_data1 =
                    output_binding_status_data1(binding, color_mappings).unwrap_or((176, 0));
                let mut status_d1_hovered = false;
                let mut status_d1_dragged = false;
                ui.horizontal(|ui| {
                    ui.label("Status");
                    let r = ui.add(DragValue::new(&mut status_data1.0).range(0..=255));
                    if r.changed()
                        && set_output_binding_status_data1(
                            binding,
                            color_mappings,
                            status_data1.0,
                            status_data1.1,
                        )
                    {
                        *dirty = true;
                    }
                    status_d1_hovered |= r.hovered();
                    status_d1_dragged |= r.dragged();
                    ui.label("Data1");
                    let r = ui.add(DragValue::new(&mut status_data1.1).range(0..=127));
                    if r.changed()
                        && set_output_binding_status_data1(
                            binding,
                            color_mappings,
                            status_data1.0,
                            status_data1.1,
                        )
                    {
                        *dirty = true;
                    }
                    status_d1_hovered |= r.hovered();
                    status_d1_dragged |= r.dragged();
                });
                value_binding_preview_active =
                    value_binding_preview_active || status_d1_hovered || status_d1_dragged;

                let mut kind = match binding.kind {
                    MidiOutputBindingKind::Value(_) => 0,
                    MidiOutputBindingKind::SceneColorValue(_) => 1,
                };

                ComboBox::new(format!("{}_output_kind_{index}", controller_id), "")
                    .selected_text(if kind == 0 {
                        "Single Value"
                    } else {
                        "Scene Color Mapping"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut kind, 0, "Single Value");
                        ui.selectable_value(&mut kind, 1, "Scene Color Mapping");
                    });

                match kind {
                    0 => {
                        if !matches!(binding.kind, MidiOutputBindingKind::Value(_)) {
                            let (status, data1) = output_binding_status_data1(binding, color_mappings)
                                .unwrap_or((176, 0));
                            binding.kind = MidiOutputBindingKind::Value(MidiValueOutput {
                                status,
                                data1,
                                ..MidiValueOutput::default()
                            });
                            *dirty = true;
                        }
                        if let MidiOutputBindingKind::Value(value) = &mut binding.kind {
                            let (preview_active, blackout_blink_preview_active) = value_output_editor(
                                ui,
                                value,
                                dirty,
                                controller_id,
                                index,
                            );
                            if preview_active || value_binding_preview_active {
                                value_preview_binding_indices.insert(index);
                            }
                            if blackout_blink_preview_active {
                                blackout_blink_preview_binding_indices.insert(index);
                            }
                        }
                    }
                    _ => {
                        if !matches!(binding.kind, MidiOutputBindingKind::SceneColorValue(_)) {
                            let (status, data1) = output_binding_status_data1(binding, color_mappings)
                                .unwrap_or((144, 0));
                            binding.kind = MidiOutputBindingKind::SceneColorValue(
                                crate::storage::asset::midi_controller::MidiSceneColorValueOutput {
                                    status,
                                    data1,
                                    ..crate::storage::asset::midi_controller::MidiSceneColorValueOutput::default()
                                },
                            );
                            *dirty = true;
                        }
                        if let MidiOutputBindingKind::SceneColorValue(scene_color_value) =
                            &mut binding.kind
                        {
                            scene_color_value_output_editor(
                                ui,
                                scene_color_value,
                                color_mappings,
                                test_device_selected,
                                dirty,
                                controller_id,
                                index,
                            );
                        }
                    }
                }
            })
            .response;
        if response.hovered() {
            *hovered_status_data1_next = status_data1;
        }
    }

    if !duplicate_outputs.is_empty() {
        bindings.extend(duplicate_outputs);
        *dirty = true;
    }

    remove_output
}
