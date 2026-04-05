use crate::{
    midi::runtime::TestCommand,
    storage::asset::midi_controller::MidiController,
};
use egui::{ComboBox, Ui};
use egui_phosphor_icons::icons;

use super::{iconized, monitor, MidiControllerTestState};

pub(super) fn render_test_device_box(
    ui: &mut Ui,
    test_state: &mut MidiControllerTestState,
    diagnostics: &[monitor::MidiPortDiagnostics],
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    test_command_sender: &kanal::Sender<TestCommand>,
) -> Option<String> {
    let mut selected_test_port = None;
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
                            let _ = test_command_sender.send(TestCommand::UnsetTestDevice);
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
                                selected_test_port = Some(diag.port_name.clone());
                                let _ = test_command_sender.send(TestCommand::SetTestDevice {
                                    port_name: diag.port_name.clone(),
                                });
                            }
                        }
                    });
            });
        });

    selected_test_port
}
