use crate::{
    storage::asset::{
        Asset,
        output_device::{OutputDevice, enttec_usb_pro::serial_numbers},
    },
    ui::{
        asset_tree::{AssetTree, TreeSelection},
        viewport_builder::default_viewport_builder,
    },
};
use egui::{ComboBox, Id, TextEdit, Ui, Vec2, ViewportId};
use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};

#[derive(Clone, Copy, Debug, IntoStaticStr, PartialEq, Eq, EnumIter)]
pub enum OutputDeviceKind {
    Artnet,
    EnttecDmxUsbPro,
}

impl OutputDevice {
    pub fn kind(&self) -> OutputDeviceKind {
        match self {
            OutputDevice::Artnet { .. } => OutputDeviceKind::Artnet,
            OutputDevice::EnttecDmxUsbPro { .. } => OutputDeviceKind::EnttecDmxUsbPro,
        }
    }
}

#[derive(Default)]
pub struct OutputDevicesWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<OutputDevice>,
    device_strings: DeviceStrings,
}

#[derive(Default)]
struct DeviceStrings {
    ip: Option<String>,
    universes: Option<String>,
}

impl OutputDevicesWindow {
    pub fn update(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("output devices window")),
            default_viewport_builder()
                .with_title("Gled: Output Devices")
                .with_inner_size(Vec2::new(500.0, 500.0))
                .with_min_inner_size(Vec2::new(500.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                egui::SidePanel::left("output devices tree")
                    .exact_width(200.0)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if self
                            .tree
                            .show(ui, ui.make_persistent_id("output_devices_tree"))
                        {
                            self.dirty = false;
                            self.device_strings = Default::default();
                        }
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    self.tree.common_settings(ui, &mut self.dirty);
                    if let TreeSelection::Asset(output_device) = &mut self.tree.selected() {
                        output_device_editor(
                            ui,
                            output_device,
                            &mut self.dirty,
                            &mut self.device_strings,
                        );
                    }
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}

fn output_device_editor(
    ui: &mut Ui,
    output_device: &mut Asset<OutputDevice>,
    dirty: &mut bool,
    device_strings: &mut DeviceStrings,
) {
    let mut kind = output_device.data.kind();
    let id = output_device.id;

    ui.label("Kind");
    ComboBox::new(format!("{id}_kind"), "")
        .selected_text({
            let name: &'static str = kind.into();
            name
        })
        .width(290.0)
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
                        OutputDeviceKind::EnttecDmxUsbPro => {
                            output_device.data = OutputDevice::EnttecDmxUsbPro {
                                serial_number: String::new(),
                            }
                        }
                    }
                    *device_strings = Default::default();
                    *dirty = true;
                }
            }
        });

    if let Some(ip) = output_device.data.ip() {
        ui.heading("IP-Address");
        let ip = device_strings.ip.get_or_insert_with(|| ip.to_string());

        if ui.add(TextEdit::singleline(ip)).changed() {
            if let Ok(ip) = ip.parse() {
                output_device.data.set_ip(ip);
                *dirty = true;
            }
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
            output_device.data.set_univeres(universes);
            *dirty = true;
        }
    }

    if let OutputDevice::EnttecDmxUsbPro { serial_number } = &output_device.data {
        ui.heading("Serial Number:");
        let mut serial_number = serial_number.clone();
        ComboBox::new(format!("{id}_serial_number"), "")
            .selected_text(serial_number.clone())
            .width(290.0)
            .show_ui(ui, |ui| {
                for sn in serial_numbers() {
                    if ui
                        .selectable_value(&mut serial_number, sn.clone(), sn)
                        .changed()
                    {
                        output_device.data = OutputDevice::EnttecDmxUsbPro {
                            serial_number: serial_number.clone(),
                        };
                        *dirty = true;
                    }
                }
            });
    }
}
