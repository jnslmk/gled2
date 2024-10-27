use egui::{Button, Color32, ComboBox, TextEdit};
use egui_extras::{Column, TableBuilder};
use std::collections::HashMap;
use strum::IntoEnumIterator;
use uuid::Uuid;

use crate::{
    extract_output::ExtractOutput,
    project::{OutputDevice, OutputDeviceKind},
};

#[derive(Default)]
pub struct OutputDevicesWindow {
    open: bool,
    device_strings: HashMap<Uuid, DeviceStrings>,
}

#[derive(Default)]
struct DeviceStrings {
    name: Option<String>,
    ip: Option<String>,
    port: Option<String>,
    universes: Option<String>,
    start: Option<String>,
}

impl OutputDevicesWindow {
    pub fn update(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        egui::Window::new("Output Devices")
            .open(&mut self.open)
            .collapsible(false)
            .min_width(600.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .show(ctx, |ui| {
                ui.vertical_centered_justified(|ui| {
                    if ui.button("Add device").clicked() {
                        let mut devices = ExtractOutput::get().devices.lock();
                        devices.insert(
                            Uuid::new_v4(),
                            OutputDevice::Artnet {
                                name: "New Artnet device".into(),
                                ip: [127, 0, 0, 1].into(),
                                universes: vec![],
                            },
                        );
                    }
                });
                TableBuilder::new(ui)
                    .column(Column::exact(100.0))
                    .column(Column::exact(150.0))
                    .column(Column::exact(100.0))
                    .column(Column::remainder())
                    .column(Column::exact(20.0))
                    .striped(true)
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.heading("Kind");
                        });
                        header.col(|ui| {
                            ui.heading("Name");
                        });
                        header.col(|ui| {
                            ui.heading("IP");
                        });
                        header.col(|_ui| {});
                        header.col(|_ui| {});
                    })
                    .body(|mut body| {
                        let extract_output = ExtractOutput::get();
                        let mut devices = extract_output.devices.lock();
                        let mut remove = None;

                        for (id, device) in devices.iter_mut() {
                            let mut kind = device.kind();

                            body.row(30.0, |mut row| {
                                row.col(|ui| {
                                    ComboBox::new(format!("{id}_kind"), "Kind")
                                        .selected_text({
                                            let name: &'static str = kind.into();
                                            name
                                        })
                                        .width(100.0)
                                        .show_ui(ui, |ui| {
                                            for k in OutputDeviceKind::iter() {
                                                if ui
                                                    .selectable_value(&mut kind, k, {
                                                        let name: &'static str = k.into();
                                                        name
                                                    })
                                                    .changed()
                                                {
                                                    match kind {
                                                        OutputDeviceKind::Artnet => {
                                                            *device = OutputDevice::Artnet {
                                                                name: "New Artnet device".into(),
                                                                ip: [127, 0, 0, 1].into(),
                                                                universes: vec![],
                                                            }
                                                        }
                                                        OutputDeviceKind::WledDRGB => {
                                                            *device = OutputDevice::WledDRGB {
                                                                name: "New Wled DRGB device".into(),
                                                                ip: [127, 0, 0, 1].into(),
                                                                port: 1324,
                                                            }
                                                        }
                                                        OutputDeviceKind::WledDNRGB => {
                                                            *device = OutputDevice::WledDNRGB {
                                                                name: "New Wled DNRGB device"
                                                                    .into(),
                                                                ip: [127, 0, 0, 1].into(),
                                                                port: 1324,
                                                                start: 0,
                                                            }
                                                        }
                                                    }

                                                    self.device_strings.remove(id);
                                                }
                                            }
                                        });
                                });

                                let device_strings = self.device_strings.entry(*id).or_default();

                                row.col(|ui| {
                                    let name = device_strings
                                        .name
                                        .get_or_insert_with(|| device.name().to_string());
                                    if ui.add(TextEdit::singleline(name)).changed() {
                                        device.set_name(name.clone());
                                    }
                                });
                                row.col(|ui| {
                                    let ip = device_strings
                                        .ip
                                        .get_or_insert_with(|| device.ip().to_string());

                                    if ui.add(TextEdit::singleline(ip)).changed() {
                                        if let Ok(ip) = ip.parse() {
                                            device.set_ip(ip);
                                        }
                                    }
                                });
                                row.col(|ui| {
                                    ui.horizontal(|ui| {
                                        if let OutputDevice::Artnet { universes, .. } = device {
                                            ui.label("Universes:");
                                            let universes =
                                                device_strings.universes.get_or_insert_with(|| {
                                                    universes
                                                        .iter()
                                                        .map(|universe| universe.to_string())
                                                        .collect::<Vec<_>>()
                                                        .join(", ")
                                                });

                                            if ui.add(TextEdit::singleline(universes)).changed() {
                                                let universes = universes
                                                    .split(',')
                                                    .map(|universe| {
                                                        universe.trim().parse().unwrap_or_default()
                                                    })
                                                    .collect();
                                                *device = OutputDevice::Artnet {
                                                    name: device.name().to_string(),
                                                    ip: device.ip(),
                                                    universes,
                                                };
                                            }
                                        }

                                        if let OutputDevice::WledDRGB { port, .. }
                                        | OutputDevice::WledDNRGB { port, .. } = device
                                        {
                                            ui.label("Port:");
                                            let port = device_strings
                                                .port
                                                .get_or_insert_with(|| port.to_string());

                                            if ui.add(TextEdit::singleline(port)).changed() {
                                                if let Ok(port) = port.parse() {
                                                    match device.clone() {
                                                        OutputDevice::WledDRGB {
                                                            name, ip, ..
                                                        } => {
                                                            *device = OutputDevice::WledDRGB {
                                                                name,
                                                                ip,
                                                                port,
                                                            };
                                                        }
                                                        OutputDevice::WledDNRGB {
                                                            name,
                                                            ip,
                                                            start,
                                                            ..
                                                        } => {
                                                            *device = OutputDevice::WledDNRGB {
                                                                name,
                                                                ip,
                                                                port,
                                                                start,
                                                            };
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }

                                        if let OutputDevice::WledDNRGB {
                                            start,
                                            port,
                                            ip,
                                            name,
                                            ..
                                        } = device
                                        {
                                            ui.label("Start:");
                                            let start = device_strings
                                                .start
                                                .get_or_insert_with(|| start.to_string());

                                            if ui.add(TextEdit::singleline(start)).changed() {
                                                if let Ok(start) = start.parse() {
                                                    *device = OutputDevice::WledDNRGB {
                                                        name: name.clone(),
                                                        ip: *ip,
                                                        port: *port,
                                                        start,
                                                    };
                                                }
                                            }
                                        }
                                    });
                                });
                                row.col(|ui| {
                                    if ui.add(Button::new("🗙").fill(Color32::DARK_RED)).clicked()
                                    {
                                        remove = Some(*id);
                                    }
                                });
                            });
                        }

                        if let Some(id) = remove {
                            self.device_strings.remove(&id);
                            devices.remove(&id);
                        }
                    });
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
