use crate::midi::monitor;
use chrono::{Local, TimeZone};
use egui::Ui;
use egui_phosphor_icons::icons;
use std::collections::VecDeque;

use super::iconized;

pub(super) fn show_live_midi_monitor(
    ui: &mut Ui,
    events: &mut VecDeque<monitor::MidiMonitorEvent>,
    diagnostics: &[monitor::MidiPortDiagnostics],
) {
    ui.horizontal(|ui| {
        ui.label(icons::GAUGE);
        ui.heading("Diagnostics");
    });

    let diagnostics_height = ui.text_style_height(&egui::TextStyle::Body) * 3.0 + 8.0;
    egui::ScrollArea::vertical()
        .max_height(diagnostics_height)
        .id_salt("midi_diagnostics_scroll")
        .show(ui, |ui| {
            if diagnostics.is_empty() {
                ui.label("No MIDI ports observed yet.");
                return;
            }

            for diag in diagnostics {
                ui.label(format!(
                    "{} | in:{} out:{} | msgs:{} | in_fail:{} out_fail:{}",
                    diag.port_name,
                    if diag.input_connected { "yes" } else { "no" },
                    if diag.output_connected { "yes" } else { "no" },
                    diag.message_count,
                    diag.input_connect_failed,
                    diag.output_connect_failed
                ));
            }
        });
    ui.separator();

    ui.horizontal(|ui| {
        ui.label(icons::WAVEFORM);
        ui.heading("Live MIDI Monitor");
        if ui.button(iconized(ui, icons::TRASH, " Clear")).clicked() {
            events.clear();
        }
    });

    let events_slice: &[_] = events.make_contiguous();
    if events_slice.is_empty() {
        ui.label("No MIDI messages yet. Move a control on your controller.");
        return;
    }

    egui::ScrollArea::vertical()
        .max_height(ui.available_height())
        .stick_to_bottom(true)
        .id_salt("live_midi_monitor_scroll")
        .show(ui, |ui| {
            let log_text = events_slice
                .iter()
                .map(|event| {
                    let bytes = if event.bytes.is_empty() {
                        "-".to_owned()
                    } else {
                        event
                            .bytes
                            .iter()
                            .map(|b| format!("{b:03}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    let time = Local
                        .timestamp_millis_opt(event.timestamp_ms as i64)
                        .single()
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                        .unwrap_or_else(|| event.timestamp_ms.to_string());
                    format!(
                        "{} | {} | {} | [{}]",
                        time, event.port_name, event.label, bytes
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            ui.add(egui::Label::new(log_text).selectable(true).wrap());
        });
}
