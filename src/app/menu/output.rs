use crate::{
    app::{svg::universes, App},
    project::{Output, OutputKind, UniverseOutput},
};
use egui::{Context, Layout, RichText, TextEdit};
use std::net::IpAddr;

impl App {
    pub fn config_output_window(&mut self, ctx: &Context) {
        if self.config_output_window_open {
            egui::Window::new("Config Outputs")
                .collapsible(false)
                .resizable(true)
                .default_pos(ctx.available_rect().center())
                .open(&mut self.config_output_window_open)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_source("output_scroll")
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                                let mut new_outputs = None;
                                {
                                    let outputs = self
                                        .extract_output
                                        .outputs
                                        .read()
                                        .expect("outputs is poisoned");

                                    let mut default_output_kind = outputs.default_output().kind();
                                    ui.label(RichText::new("Default Output").heading());
                                    ui.horizontal(|ui| {
                                        if ui
                                            .radio_value(
                                                &mut default_output_kind,
                                                OutputKind::Artnet,
                                                "Artnet",
                                            )
                                            .changed()
                                        {
                                            let mut outputs = outputs.clone();
                                            *outputs.default_output_mut() = Output::Artnet {
                                                ip: "127.0.0.1"
                                                    .parse()
                                                    .expect("Could not parse 127.0.0.1"),
                                            };
                                            new_outputs = Some(outputs);
                                        };
                                        if ui
                                            .radio_value(
                                                &mut default_output_kind,
                                                OutputKind::WledDRGB,
                                                "Wled DRGB",
                                            )
                                            .changed()
                                        {
                                            let mut outputs = outputs.clone();
                                            *outputs.default_output_mut() = Output::WledDRGB {
                                                ip: "127.0.0.1"
                                                    .parse()
                                                    .expect("Could not parse 127.0.0.1"),
                                                port: 21324,
                                            };
                                            new_outputs = Some(outputs);
                                        };
                                        if ui
                                            .radio_value(
                                                &mut default_output_kind,
                                                OutputKind::WledDNRGB,
                                                "Wled DNRGB",
                                            )
                                            .changed()
                                        {
                                            let mut outputs = outputs.clone();
                                            *outputs.default_output_mut() = Output::WledDNRGB {
                                                ip: "127.0.0.1"
                                                    .parse()
                                                    .expect("Could not parse 127.0.0.1"),
                                                port: 21324,
                                                start: 0,
                                            };
                                            new_outputs = Some(outputs);
                                        };
                                    });
                                    match outputs.default_output() {
                                        Output::Artnet { ip } => {
                                            ui.label("Artnet IP");
                                            let ip = self
                                                .inputs
                                                .entry("default_artnet_ip".to_string())
                                                .or_insert_with(|| ip.to_string());
                                            if ui.add(TextEdit::singleline(ip)).changed()
                                                && ip.parse::<IpAddr>().is_ok()
                                            {
                                                let mut outputs = outputs.clone();
                                                *outputs.default_output_mut() = Output::Artnet {
                                                    ip: ip.parse().expect("Should never happen"),
                                                };
                                                new_outputs = Some(outputs);
                                            }
                                        }
                                        Output::WledDRGB { ip, port } => {
                                            {
                                                ui.label("Wled DRGB IP");
                                                let ip = self
                                                    .inputs
                                                    .entry("default_wledrgb_ip".to_string())
                                                    .or_insert_with(|| ip.to_string());
                                                if ui.add(TextEdit::singleline(ip)).changed()
                                                    && ip.parse::<IpAddr>().is_ok()
                                                {
                                                    let mut outputs = outputs.clone();
                                                    *outputs.default_output_mut() =
                                                        Output::WledDRGB {
                                                            ip: ip
                                                                .parse()
                                                                .expect("Should never happen"),
                                                            port: *port,
                                                        };
                                                    new_outputs = Some(outputs);
                                                }
                                            }

                                            {
                                                ui.label("Wled DRGB Port");
                                                let port = self
                                                    .inputs
                                                    .entry("default_wledrgb_port".to_string())
                                                    .or_insert_with(|| port.to_string());
                                                if ui.add(TextEdit::singleline(port)).changed()
                                                    && port.parse::<u16>().is_ok()
                                                {
                                                    let mut outputs = outputs.clone();
                                                    *outputs.default_output_mut() =
                                                        Output::WledDRGB {
                                                            ip: *ip,
                                                            port: port
                                                                .parse()
                                                                .expect("Should never happen"),
                                                        };
                                                    new_outputs = Some(outputs);
                                                }
                                            }
                                        }
                                        Output::WledDNRGB { ip, port, start } => {
                                            {
                                                ui.label("Wled DNRGB IP");
                                                let ip = self
                                                    .inputs
                                                    .entry("default_wlednrgb_ip".to_string())
                                                    .or_insert_with(|| ip.to_string());
                                                if ui.add(TextEdit::singleline(ip)).changed()
                                                    && ip.parse::<IpAddr>().is_ok()
                                                {
                                                    let mut outputs = outputs.clone();
                                                    *outputs.default_output_mut() =
                                                        Output::WledDNRGB {
                                                            ip: ip
                                                                .parse()
                                                                .expect("Should never happen"),
                                                            port: *port,
                                                            start: *start,
                                                        };
                                                    new_outputs = Some(outputs);
                                                }
                                            }

                                            {
                                                ui.label("Wled DNRGB Port");
                                                let port = self
                                                    .inputs
                                                    .entry("default_wlednrgb_port".to_string())
                                                    .or_insert_with(|| port.to_string());
                                                if ui.add(TextEdit::singleline(port)).changed()
                                                    && port.parse::<u16>().is_ok()
                                                {
                                                    let mut outputs = outputs.clone();
                                                    *outputs.default_output_mut() =
                                                        Output::WledDNRGB {
                                                            ip: *ip,
                                                            port: port
                                                                .parse()
                                                                .expect("Should never happen"),
                                                            start: *start,
                                                        };
                                                    new_outputs = Some(outputs);
                                                }
                                            }

                                            {
                                                ui.label("Wled DNRGB Start");
                                                let start = self
                                                    .inputs
                                                    .entry("default_wlednrgb_start".to_string())
                                                    .or_insert_with(|| start.to_string());
                                                if ui.add(TextEdit::singleline(start)).changed()
                                                    && start.parse::<u16>().is_ok()
                                                {
                                                    let mut outputs = outputs.clone();
                                                    *outputs.default_output_mut() =
                                                        Output::WledDNRGB {
                                                            ip: *ip,
                                                            port: *port,
                                                            start: start
                                                                .parse()
                                                                .expect("Should never happen"),
                                                        };
                                                    new_outputs = Some(outputs);
                                                }
                                            }
                                        }
                                    }

                                    for universe in universes() {
                                        ui.separator();
                                        ui.label(
                                            RichText::new(format!("Universe: {universe}"))
                                                .heading(),
                                        );
                                        let universe_output = outputs.universe_output(universe);
                                        let mut universe_output_kind = universe_output.kind();
                                        ui.horizontal(|ui| {
                                            if ui
                                                .radio_value(
                                                    &mut universe_output_kind,
                                                    OutputKind::Default,
                                                    "Default",
                                                )
                                                .changed()
                                            {
                                                let mut outputs = outputs.clone();
                                                *outputs.universe_output_mut(universe) =
                                                    UniverseOutput::Default;
                                                new_outputs = Some(outputs);
                                            }
                                            if ui
                                                .radio_value(
                                                    &mut universe_output_kind,
                                                    OutputKind::Artnet,
                                                    "Artnet",
                                                )
                                                .changed()
                                            {
                                                let mut outputs = outputs.clone();
                                                *outputs.universe_output_mut(universe) =
                                                    UniverseOutput::Artnet {
                                                        ip: "127.0.0.1"
                                                            .parse()
                                                            .expect("Could not parse 127.0.0.1"),
                                                        universe,
                                                    };
                                                new_outputs = Some(outputs);
                                            }
                                            if ui
                                                .radio_value(
                                                    &mut universe_output_kind,
                                                    OutputKind::WledDRGB,
                                                    "Wled DRGB",
                                                )
                                                .changed()
                                            {
                                                let mut outputs = outputs.clone();
                                                *outputs.universe_output_mut(universe) =
                                                    UniverseOutput::WledDRGB {
                                                        ip: "127.0.0.1"
                                                            .parse()
                                                            .expect("Could not parse 127.0.0.1"),
                                                        port: 21324,
                                                    };
                                                new_outputs = Some(outputs);
                                            }
                                            if ui
                                                .radio_value(
                                                    &mut universe_output_kind,
                                                    OutputKind::WledDNRGB,
                                                    "Wled DNRGB",
                                                )
                                                .changed()
                                            {
                                                let mut outputs = outputs.clone();
                                                *outputs.universe_output_mut(universe) =
                                                    UniverseOutput::WledDNRGB {
                                                        ip: "127.0.0.1"
                                                            .parse()
                                                            .expect("Could not parse 127.0.0.1"),
                                                        port: 21324,
                                                        start: 0,
                                                    };
                                                new_outputs = Some(outputs);
                                            }
                                        });

                                        match universe_output {
                                            UniverseOutput::Default => (),
                                            UniverseOutput::Artnet {
                                                ip,
                                                universe: universe_changed,
                                            } => {
                                                {
                                                    ui.label("Artnet IP");
                                                    let ip = self
                                                        .inputs
                                                        .entry(format!("artnet_ip_{universe}"))
                                                        .or_insert_with(|| ip.to_string());
                                                    if ui.add(TextEdit::singleline(ip)).changed()
                                                        && ip.parse::<IpAddr>().is_ok()
                                                    {
                                                        let mut outputs = outputs.clone();
                                                        *outputs.universe_output_mut(universe) =
                                                            UniverseOutput::Artnet {
                                                                ip: ip
                                                                    .parse()
                                                                    .expect("Should never happen"),
                                                                universe: universe_changed,
                                                            };
                                                        new_outputs = Some(outputs);
                                                    }
                                                }

                                                {
                                                    ui.label("Artnet Universe");
                                                    let universe_str = self
                                                        .inputs
                                                        .entry(format!(
                                                            "artnet_universe_{universe}"
                                                        ))
                                                        .or_insert_with(|| {
                                                            universe_changed.to_string()
                                                        });
                                                    if ui
                                                        .add(TextEdit::singleline(universe_str))
                                                        .changed()
                                                        && universe_str.parse::<u16>().is_ok()
                                                    {
                                                        let mut outputs = outputs.clone();
                                                        *outputs.universe_output_mut(universe) =
                                                            UniverseOutput::Artnet {
                                                                ip,
                                                                universe: universe_str
                                                                    .parse()
                                                                    .expect("Should never happen"),
                                                            };
                                                        new_outputs = Some(outputs);
                                                    }
                                                }
                                            }
                                            UniverseOutput::WledDRGB { ip, port } => {
                                                {
                                                    ui.label("Wled DRGB IP");
                                                    let ip = self
                                                        .inputs
                                                        .entry(format!("wledrgb_ip_{universe}"))
                                                        .or_insert_with(|| ip.to_string());
                                                    if ui.add(TextEdit::singleline(ip)).changed()
                                                        && ip.parse::<IpAddr>().is_ok()
                                                    {
                                                        let mut outputs = outputs.clone();
                                                        *outputs.universe_output_mut(universe) =
                                                            UniverseOutput::WledDRGB {
                                                                ip: ip
                                                                    .parse()
                                                                    .expect("Should never happen"),
                                                                port,
                                                            };
                                                        new_outputs = Some(outputs);
                                                    }
                                                }

                                                {
                                                    ui.label("Wled DRGB Port");
                                                    let port = self
                                                        .inputs
                                                        .entry(format!("wledrgb_port_{universe}"))
                                                        .or_insert_with(|| port.to_string());
                                                    if ui.add(TextEdit::singleline(port)).changed()
                                                        && port.parse::<u16>().is_ok()
                                                    {
                                                        let mut outputs = outputs.clone();
                                                        *outputs.universe_output_mut(universe) =
                                                            UniverseOutput::WledDRGB {
                                                                ip,
                                                                port: port
                                                                    .parse()
                                                                    .expect("Should never happen"),
                                                            };
                                                        new_outputs = Some(outputs);
                                                    }
                                                }
                                            }
                                            UniverseOutput::WledDNRGB { ip, port, start } => {
                                                {
                                                    ui.label("Wled DNRGB IP");
                                                    let ip = self
                                                        .inputs
                                                        .entry(format!("wlednrgb_ip_{universe}"))
                                                        .or_insert_with(|| ip.to_string());
                                                    if ui.add(TextEdit::singleline(ip)).changed()
                                                        && ip.parse::<IpAddr>().is_ok()
                                                    {
                                                        let mut outputs = outputs.clone();
                                                        *outputs.universe_output_mut(universe) =
                                                            UniverseOutput::WledDNRGB {
                                                                ip: ip
                                                                    .parse()
                                                                    .expect("Should never happen"),
                                                                port,
                                                                start,
                                                            };
                                                        new_outputs = Some(outputs);
                                                    }
                                                }

                                                {
                                                    ui.label("Wled DNRGB Port");
                                                    let port = self
                                                        .inputs
                                                        .entry(format!("wlednrgb_port_{universe}"))
                                                        .or_insert_with(|| port.to_string());
                                                    if ui.add(TextEdit::singleline(port)).changed()
                                                        && port.parse::<u16>().is_ok()
                                                    {
                                                        let mut outputs = outputs.clone();
                                                        *outputs.universe_output_mut(universe) =
                                                            UniverseOutput::WledDNRGB {
                                                                ip,
                                                                port: port
                                                                    .parse()
                                                                    .expect("Should never happen"),
                                                                start,
                                                            };
                                                        new_outputs = Some(outputs);
                                                    }
                                                }

                                                {
                                                    ui.label("Wled DNRGB Start");
                                                    let start = self
                                                        .inputs
                                                        .entry(format!("wlednrgb_start_{universe}"))
                                                        .or_insert_with(|| start.to_string());
                                                    if ui.add(TextEdit::singleline(start)).changed()
                                                        && start.parse::<u16>().is_ok()
                                                    {
                                                        let mut outputs = outputs.clone();
                                                        *outputs.universe_output_mut(universe) =
                                                            UniverseOutput::WledDNRGB {
                                                                ip,
                                                                port,
                                                                start: start
                                                                    .parse()
                                                                    .expect("Should never happen"),
                                                            };
                                                        new_outputs = Some(outputs);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(outputs) = new_outputs {
                                    *self
                                        .extract_output
                                        .outputs
                                        .write()
                                        .expect("outputs is poisoned") = outputs;
                                }
                            });
                        });
                });
        }
    }
}
