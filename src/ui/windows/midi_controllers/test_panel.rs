use crate::{
    midi::runtime::{self, RuntimeTestState},
    storage::asset::{midi_controller::MidiController, midi_controller::MidiNamedSceneColorMapping},
};
use egui::{ComboBox, Ui};
use egui_phosphor_icons::icons;
use std::collections::HashMap;

use super::{iconized, monitor, MidiControllerTestState};

pub(super) fn sync_runtime_test_state(
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    color_mappings: &[MidiNamedSceneColorMapping],
    test_state: &MidiControllerTestState,
) {
    let Some(selected_output_port) = test_state.selected_output_port.clone() else {
        runtime::set_controller_test_state(controller_id, None);
        return;
    };

    if selected_output_port.is_empty() {
        runtime::set_controller_test_state(controller_id, None);
        return;
    }

    let mut scene_color_value_overrides = HashMap::new();
    for (&mapping_index, value) in &test_state.color_overrides_by_mapping_index {
        if let Some(mapping) = color_mappings.get(mapping_index) {
            scene_color_value_overrides.insert(mapping.name.clone(), *value);
        }
    }

    runtime::set_controller_test_state(
        controller_id,
        Some(RuntimeTestState {
            selected_output_port: Some(selected_output_port),
            value_output_overrides: test_state.value_output_overrides.clone(),
            scene_color_value_overrides,
        }),
    );
}

pub(super) fn render_test_device_box(
    ui: &mut Ui,
    test_state: &mut MidiControllerTestState,
    diagnostics: &[monitor::MidiPortDiagnostics],
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
) {
    egui::Frame::group(ui.style())
        .fill(ui.visuals().extreme_bg_color)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(iconized(ui, icons::RADIO_BUTTON, " Test Device"));
                ComboBox::new(format!("{}_test_output_port", controller_id), "")
                    .selected_text(if test_state.selected_output_port.is_none() {
                        "No Test Device"
                    } else {
                        test_state.selected_output_port.as_deref().unwrap_or("No Test Device")
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(test_state.selected_output_port.is_none(), "No Test Device")
                            .clicked()
                        {
                            test_state.selected_output_port = None;
                        }
                        for diag in diagnostics {
                            if !diag.output_connected {
                                continue;
                            }
                            if ui
                                .selectable_label(
                                    test_state
                                        .selected_output_port
                                        .as_deref()
                                        .is_some_and(|port| port == diag.port_name),
                                    &diag.port_name,
                                )
                                .clicked()
                            {
                                test_state.selected_output_port = Some(diag.port_name.clone());
                            }
                        }
                    });
            });
        });
}
