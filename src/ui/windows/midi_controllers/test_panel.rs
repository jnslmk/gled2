use crate::{midi::runtime::TestCommand, storage::asset::midi_controller::MidiController};
use egui::{ComboBox, Ui};
use egui_phosphor_icons::icons;
use midir::MidiOutput;
use std::collections::BTreeSet;

use super::{MidiControllerTestState, iconized, monitor};

fn is_selectable_output_port(name: &str) -> bool {
    !name.trim().is_empty()
        && !name.contains("gled_read_input")
        && !name.contains("gled_write_output")
        && !name.contains("Midi Through")
}

fn available_output_ports() -> Vec<String> {
    let Ok(output) = MidiOutput::new("gled_test_device_scan") else {
        return Vec::new();
    };

    output
        .ports()
        .into_iter()
        .filter_map(|port| output.port_name(&port).ok())
        .filter(|name| is_selectable_output_port(name))
        .collect()
}

fn test_device_port_options(diagnostics: &[monitor::MidiPortDiagnostics]) -> Vec<String> {
    let mut ports = BTreeSet::new();

    for diag in diagnostics {
        if diag.output_connected && is_selectable_output_port(&diag.port_name) {
            ports.insert(diag.port_name.clone());
        }
    }
    for port in available_output_ports() {
        ports.insert(port);
    }

    ports.into_iter().collect()
}

pub(super) fn render_test_device_box(
    ui: &mut Ui,
    test_state: &mut MidiControllerTestState,
    diagnostics: &[monitor::MidiPortDiagnostics],
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    test_command_sender: &kanal::Sender<TestCommand>,
) -> Option<String> {
    let mut selected_test_port = None;
    let port_options = test_device_port_options(diagnostics);
    egui::Frame::group(ui.style())
        .fill(ui.visuals().extreme_bg_color)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(iconized(ui, icons::RADIO_BUTTON, " Test Device"));
                ComboBox::new(format!("{}_test_output_port", controller_id), "")
                    .selected_text(if test_state.selected_output_port.is_none() {
                        "No Test Device"
                    } else {
                        test_state
                            .selected_output_port
                            .as_deref()
                            .unwrap_or("No Test Device")
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                test_state.selected_output_port.is_none(),
                                "No Test Device",
                            )
                            .clicked()
                        {
                            test_state.selected_output_port = None;
                            let _ = test_command_sender.send(TestCommand::UnsetTestDevice);
                        }
                        for port_name in &port_options {
                            if ui
                                .selectable_label(
                                    test_state
                                        .selected_output_port
                                        .as_deref()
                                        .is_some_and(|port| port == port_name),
                                    port_name,
                                )
                                .clicked()
                            {
                                test_state.selected_output_port = Some(port_name.clone());
                                selected_test_port = Some(port_name.clone());
                                let _ = test_command_sender.send(TestCommand::SetTestDevice {
                                    port_name: port_name.clone(),
                                });
                            }
                        }
                    });
            });
        });

    selected_test_port
}
