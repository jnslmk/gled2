use crate::{
    storage::{Asset, AssetId, OutputDevice},
    ui::asset_tree::{AssetTree, TreeSelection},
};
use egui::{Button, ComboBox, Margin, TextEdit, Ui};
use std::{collections::HashMap, sync::Arc};
use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};

#[derive(Clone, Copy, Debug, IntoStaticStr, PartialEq, Eq, EnumIter)]
pub enum OutputDeviceKind {
    Artnet,
    WledDRGB,
    WledDNRGB,
}

impl OutputDevice {
    pub fn kind(&self) -> OutputDeviceKind {
        match self {
            OutputDevice::Artnet { .. } => OutputDeviceKind::Artnet,
            OutputDevice::WledDRGB { .. } => OutputDeviceKind::WledDRGB,
            OutputDevice::WledDNRGB { .. } => OutputDeviceKind::WledDNRGB,
        }
    }
}

#[derive(Default)]
pub struct OutputDevicesWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<OutputDevice>,
    device_strings: HashMap<AssetId<OutputDevice>, DeviceStrings>,
}

#[derive(Default)]
struct DeviceStrings {
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
                egui::SidePanel::left("output devices tree")
                    .exact_width(300.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        self.tree
                            .show(ui, ui.make_persistent_id("output_devices_tree"));
                    });

                egui::Frame::default().outer_margin(Margin::same(4.0)).show(
                    ui,
                    |ui| match &mut self.tree.selected() {
                        TreeSelection::None => {
                            ui.label("Please select an item from the tree");
                        }
                        TreeSelection::Asset(palette) => {
                            output_device_editor(
                                ui,
                                palette,
                                &mut self.dirty,
                                &mut self.device_strings,
                            );
                            self.tree.show_delete_button(ui);
                        }
                        TreeSelection::Dir { .. } => {
                            self.tree.show_folder_editor(ui);
                        }
                    },
                );
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}

fn output_device_editor(
    ui: &mut Ui,
    output_device: &mut Asset<OutputDevice>,
    dirty: &mut bool,
    device_strings: &mut HashMap<AssetId<OutputDevice>, DeviceStrings>,
) {
    let mut kind = output_device.data.kind();
    let id = output_device.id;

    ui.label("Name:");
    let mut name = output_device.name().to_string();
    let res = ui.text_edit_singleline(&mut name);
    if res.changed() {
        *dirty = true;
        output_device.path.pop();
        output_device.path.push(name);
    }

    ui.label("Kind");
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
                            output_device.data = OutputDevice::Artnet {
                                ip: [127, 0, 0, 1].into(),
                                universes: vec![],
                            }
                        }
                        OutputDeviceKind::WledDRGB => {
                            output_device.data = OutputDevice::WledDRGB {
                                ip: [127, 0, 0, 1].into(),
                                port: 1324,
                            }
                        }
                        OutputDeviceKind::WledDNRGB => {
                            output_device.data = OutputDevice::WledDNRGB {
                                ip: [127, 0, 0, 1].into(),
                                port: 1324,
                                start: 0,
                            }
                        }
                    }
                    device_strings.remove(&id);
                    *dirty = true;
                }
            }
        });

    let device_strings = device_strings.entry(id).or_default();

    ui.heading("IP-Address");
    let ip = device_strings
        .ip
        .get_or_insert_with(|| output_device.data.ip().to_string());

    if ui.add(TextEdit::singleline(ip)).changed() {
        if let Ok(ip) = ip.parse() {
            output_device.data.set_ip(ip);
            *dirty = true;
        }
    }

    if let OutputDevice::Artnet { universes, .. } = &output_device.data {
        ui.heading("Universes:");
        let universes = device_strings.universes.get_or_insert_with(|| {
            universes
                .iter()
                .map(|universe| universe.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        });

        if ui.add(TextEdit::singleline(universes)).changed() {
            let universes = universes
                .split(',')
                .map(|universe| universe.trim().parse().unwrap_or_default())
                .collect();
            output_device.data = OutputDevice::Artnet {
                ip: output_device.data.ip(),
                universes,
            };
            *dirty = true;
        }
    }

    if let OutputDevice::WledDRGB { port, .. } | OutputDevice::WledDNRGB { port, .. } =
        output_device.data
    {
        ui.heading("Port:");
        let port = device_strings.port.get_or_insert_with(|| port.to_string());

        if ui.add(TextEdit::singleline(port)).changed() {
            if let Ok(port) = port.parse() {
                match output_device.data.clone() {
                    OutputDevice::WledDRGB { ip, .. } => {
                        output_device.data = OutputDevice::WledDRGB { ip, port };
                    }
                    OutputDevice::WledDNRGB { ip, start, .. } => {
                        output_device.data = OutputDevice::WledDNRGB { ip, port, start };
                    }
                    _ => {}
                }
            }

            *dirty = true;
        }
    }

    if let OutputDevice::WledDNRGB {
        start, port, ip, ..
    } = &output_device.data
    {
        ui.heading("Start:");
        let start = device_strings
            .start
            .get_or_insert_with(|| start.to_string());

        if ui.add(TextEdit::singleline(start)).changed() {
            if let Ok(start) = start.parse() {
                output_device.data = OutputDevice::WledDNRGB {
                    ip: *ip,
                    port: *port,
                    start,
                };
            }
        }
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(*dirty, Button::new("Save"))
            .on_hover_ui(|ui| {
                ui.label("Save output device to disk");
            })
            .clicked()
        {
            *dirty = false;
            output_device.clone().save();
        }
        if ui
            .add_enabled(*dirty, Button::new("Reset"))
            .on_hover_ui(|ui| {
                ui.label("Reset to state on disk");
            })
            .clicked()
        {
            *dirty = false;
            *output_device = Arc::unwrap_or_clone(Asset::get(output_device.id).unwrap_or_default());
        }
    });
}
