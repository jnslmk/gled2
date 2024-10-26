use crate::{
    app::svg::universes,
    extract_output::ExtractOutput,
    project::{OutputKind, UniverseOutput},
};
use egui::{Context, Layout, RichText, TextEdit};
use std::collections::HashMap;

#[derive(Default)]
pub struct OutputWindow {
    open: bool,
    universes_strings: HashMap<u16, UniverseStrings>,
}

#[derive(Default)]
struct UniverseStrings {
    artnet_ip: Option<String>,
    artnet_universe: Option<String>,
    wled_drgb_ip: Option<String>,
    wled_drgb_port: Option<String>,
    wled_dnrgb_ip: Option<String>,
    wled_dnrgb_port: Option<String>,
    wled_dnrgb_start: Option<String>,
}

impl OutputWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        egui::Window::new("Config Outputs")
            .collapsible(false)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("output_scroll")
                    .show(ui, |ui| {
                        let extract_output = ExtractOutput::get();
                        let mut outputs = extract_output.outputs.lock();

                        ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                            let universes = universes();
                            extract_output
                                .universes
                                .lock()
                                .retain(|universe| universes.contains(universe));

                            for universe in universes {
                                let universe_strings =
                                    self.universes_strings.entry(universe).or_default();

                                ui.label(RichText::new(format!("Universe: {universe}")).heading());
                                let universe_output = outputs.universe_output(universe);
                                let mut universe_output_kind = universe_output.kind();
                                ui.horizontal(|ui| {
                                    if ui
                                        .radio_value(
                                            &mut universe_output_kind,
                                            OutputKind::Artnet,
                                            "Artnet",
                                        )
                                        .changed()
                                    {
                                        *universe_output = UniverseOutput::Artnet {
                                            ip: "127.0.0.1"
                                                .parse()
                                                .expect("Could not parse 127.0.0.1"),
                                            universe,
                                        };
                                    }
                                    if ui
                                        .radio_value(
                                            &mut universe_output_kind,
                                            OutputKind::WledDRGB,
                                            "Wled DRGB",
                                        )
                                        .changed()
                                    {
                                        *universe_output = UniverseOutput::WledDRGB {
                                            ip: "127.0.0.1"
                                                .parse()
                                                .expect("Could not parse 127.0.0.1"),
                                            port: 21324,
                                        };
                                    }
                                    if ui
                                        .radio_value(
                                            &mut universe_output_kind,
                                            OutputKind::WledDNRGB,
                                            "Wled DNRGB",
                                        )
                                        .changed()
                                    {
                                        *universe_output = UniverseOutput::WledDNRGB {
                                            ip: "127.0.0.1"
                                                .parse()
                                                .expect("Could not parse 127.0.0.1"),
                                            port: 21324,
                                            start: 0,
                                        };
                                    }
                                });

                                match universe_output.clone() {
                                    UniverseOutput::Artnet { ip, universe } => {
                                        {
                                            ui.label("Artnet IP");
                                            let ip = universe_strings
                                                .artnet_ip
                                                .get_or_insert_with(|| ip.to_string());

                                            if ui.add(TextEdit::singleline(ip)).changed() {
                                                if let Ok(ip) = ip.parse() {
                                                    *universe_output =
                                                        UniverseOutput::Artnet { ip, universe };
                                                }
                                            }
                                        }

                                        {
                                            ui.label("Artnet Universe");
                                            let universe = universe_strings
                                                .artnet_universe
                                                .get_or_insert_with(|| universe.to_string());

                                            if ui.add(TextEdit::singleline(universe)).changed() {
                                                if let Ok(universe) = universe.parse() {
                                                    *universe_output =
                                                        UniverseOutput::Artnet { ip, universe };
                                                }
                                            }
                                        }
                                    }
                                    UniverseOutput::WledDRGB { ip, port } => {
                                        {
                                            ui.label("Wled DRGB IP");
                                            let ip = universe_strings
                                                .wled_drgb_ip
                                                .get_or_insert_with(|| ip.to_string());
                                            if ui.add(TextEdit::singleline(ip)).changed() {
                                                if let Ok(ip) = ip.parse() {
                                                    *universe_output =
                                                        UniverseOutput::WledDRGB { ip, port };
                                                }
                                            }
                                        }

                                        {
                                            ui.label("Wled DRGB Port");
                                            let port = universe_strings
                                                .wled_drgb_port
                                                .get_or_insert_with(|| port.to_string());
                                            if ui.add(TextEdit::singleline(port)).changed() {
                                                if let Ok(port) = port.parse() {
                                                    *universe_output =
                                                        UniverseOutput::WledDRGB { ip, port };
                                                }
                                            }
                                        }
                                    }
                                    UniverseOutput::WledDNRGB { ip, port, start } => {
                                        {
                                            ui.label("Wled DNRGB IP");
                                            let ip = universe_strings
                                                .wled_dnrgb_ip
                                                .get_or_insert_with(|| ip.to_string());
                                            if ui.add(TextEdit::singleline(ip)).changed() {
                                                if let Ok(ip) = ip.parse() {
                                                    *universe_output = UniverseOutput::WledDNRGB {
                                                        ip,
                                                        port,
                                                        start,
                                                    };
                                                }
                                            }
                                        }

                                        {
                                            ui.label("Wled DNRGB Port");
                                            let port = universe_strings
                                                .wled_dnrgb_port
                                                .get_or_insert_with(|| port.to_string());
                                            if ui.add(TextEdit::singleline(port)).changed() {
                                                if let Ok(port) = port.parse() {
                                                    *universe_output = UniverseOutput::WledDNRGB {
                                                        ip,
                                                        port,
                                                        start,
                                                    };
                                                }
                                            }
                                        }

                                        {
                                            ui.label("Wled DNRGB Start");
                                            let start = universe_strings
                                                .wled_dnrgb_start
                                                .get_or_insert_with(|| start.to_string());
                                            if ui.add(TextEdit::singleline(start)).changed() {
                                                if let Ok(start) = start.parse() {
                                                    *universe_output = UniverseOutput::WledDNRGB {
                                                        ip,
                                                        port,
                                                        start,
                                                    };
                                                }
                                            }
                                        }
                                    }
                                }

                                ui.separator();
                            }
                        });
                    });
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
