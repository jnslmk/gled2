use crate::storage::asset::midi_controller::{
    MidiNamedSceneColorMapping, MidiSceneColorMessage, MidiSceneColorMessageMap,
    MidiSceneColorValueOutput,
};
use egui::{ComboBox, DragValue, Ui};
use egui_phosphor_icons::icons;
use std::collections::HashMap;

use super::{action_converters::scene_target_editor, iconized};

pub(super) fn scene_color_value_output_editor(
    ui: &mut Ui,
    output: &mut MidiSceneColorValueOutput,
    color_mappings: &[MidiNamedSceneColorMapping],
    test_device_selected: bool,
    dirty: &mut bool,
    controller_id: crate::storage::asset_id::AssetId<crate::storage::asset::midi_controller::MidiController>,
    index: usize,
) {
    let crate::storage::asset::midi_controller::MidiColorSource::SceneColor { target } =
        &mut output.source;
    scene_target_editor(
        ui,
        target,
        dirty,
        format!("{}_{}_scene_color_value_target", controller_id, index),
    );

    ui.horizontal(|ui| {
        ui.label(iconized(ui, icons::PALETTE, " Mapping"));
        ComboBox::new(
            format!("{}_scene_color_mapping_name_{index}", controller_id),
            "",
        )
        .selected_text(if output.mapping_name.is_empty() {
            "Select mapping"
        } else {
            output.mapping_name.as_str()
        })
        .show_ui(ui, |ui| {
            for named in color_mappings {
                if ui
                    .selectable_label(output.mapping_name == named.name, &named.name)
                    .clicked()
                {
                    output.mapping_name = named.name.clone();
                    *dirty = true;
                }
            }
        });
    });

    if output.mapping_name.is_empty() && !color_mappings.is_empty() {
        output.mapping_name = color_mappings[0].name.clone();
        *dirty = true;
    }
    if color_mappings.is_empty() {
        ui.small("Add a color mapping first.");
    } else if test_device_selected {
        ui.small("Hover or edit a color value in Color Mappings to preview on the active test device.");
    }
}

pub(super) fn render_color_mappings_section(
    ui: &mut Ui,
    mappings: &mut Vec<MidiNamedSceneColorMapping>,
    dirty: &mut bool,
    _controller_id: crate::storage::asset_id::AssetId<crate::storage::asset::midi_controller::MidiController>,
) -> HashMap<usize, u8> {
    let mut preview_overrides = HashMap::new();
    egui::CollapsingHeader::new(iconized(ui, icons::PALETTE, " Color Mappings"))
        .default_open(true)
        .show(ui, |ui| {
            if ui
                .button(iconized(ui, icons::PLUS, " Add Color Mapping"))
                .clicked()
            {
                mappings.push(MidiNamedSceneColorMapping {
                    name: format!("Mapping {}", mappings.len() + 1),
                    ..MidiNamedSceneColorMapping::default()
                });
                *dirty = true;
            }

            let mut remove_index = None;
            for (index, mapping) in mappings.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Mapping {}", index + 1));
                        if ui.button(iconized(ui, icons::TRASH, " Remove")).clicked() {
                            remove_index = Some(index);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name");
                        if ui.text_edit_singleline(&mut mapping.name).changed() {
                            *dirty = true;
                        }
                    });

                    let mut preview_value = None;
                    egui::CollapsingHeader::new("Inactive")
                        .default_open(true)
                        .show(ui, |ui| {
                            scene_color_message_map_editor(
                                ui,
                                &mut mapping.inactive,
                                &mut preview_value,
                                dirty,
                            );
                        });
                    egui::CollapsingHeader::new("Active")
                        .default_open(false)
                        .show(ui, |ui| {
                            scene_color_message_map_editor(
                                ui,
                                &mut mapping.active,
                                &mut preview_value,
                                dirty,
                            );
                        });
                    egui::CollapsingHeader::new("Flashed")
                        .default_open(false)
                        .show(ui, |ui| {
                            scene_color_message_map_editor(
                                ui,
                                &mut mapping.flashed,
                                &mut preview_value,
                                dirty,
                            );
                        });

                    if let Some(value) = preview_value {
                        preview_overrides.insert(index, value);
                    }
                });
            }

            if let Some(index) = remove_index {
                mappings.remove(index);
                *dirty = true;
            }
        });

    preview_overrides
}

fn scene_color_message_map_editor(
    ui: &mut Ui,
    mappings: &mut MidiSceneColorMessageMap,
    preview: &mut Option<u8>,
    dirty: &mut bool,
) {
    scene_color_message_editor(ui, "Red", &mut mappings.red, preview, dirty);
    scene_color_message_editor(ui, "Green", &mut mappings.green, preview, dirty);
    scene_color_message_editor(ui, "Blue", &mut mappings.blue, preview, dirty);
    scene_color_message_editor(ui, "White", &mut mappings.white, preview, dirty);
    scene_color_message_editor(ui, "Orange", &mut mappings.orange, preview, dirty);
    scene_color_message_editor(ui, "Yellow", &mut mappings.yellow, preview, dirty);
    scene_color_message_editor(ui, "Purple", &mut mappings.purple, preview, dirty);
    scene_color_message_editor(ui, "Pink", &mut mappings.pink, preview, dirty);
    scene_color_message_editor(ui, "Black", &mut mappings.black, preview, dirty);
}

fn scene_color_message_editor(
    ui: &mut Ui,
    label: &str,
    message: &mut MidiSceneColorMessage,
    preview: &mut Option<u8>,
    dirty: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label("Value");
        let response = ui.add(DragValue::new(&mut message.value).range(0..=127));
        if response.changed() {
            *dirty = true;
        }

        if preview.is_none()
            && (response.changed()
                || response.dragged()
                || response.is_pointer_button_down_on()
                || response.has_focus()
                || response.hovered())
        {
            *preview = Some(message.value);
        }
    });
}
