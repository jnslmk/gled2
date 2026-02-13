use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use crate::audio::AUDIO_DEVICES;
use crate::storage::asset::project::Project;
use crate::ui::action::UiAction;
use crate::{
    input::artnet::ARTNET_CONFIG,
    storage::asset::{Asset, output_device::routing::OutputRouting},
    ui::{
        ChangeButton,
        window_common::{default_viewport_builder, gled_window_frame},
        windows::output_routings::HOVERED_OUTPUT_ROUTING,
    },
};
use chrono::Local;
use egui::{
    Button, CentralPanel, ComboBox, Context, Id, Layout, Response, RichText, SidePanel, Slider,
    TextEdit, Ui, Vec2, ViewportId, Widget, WidgetText,
};
use egui_phosphor_icons::icons;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};

#[derive(Default)]
pub struct ExternalDeviceSettings {
    open: bool,
    selected_submenu: String,
    edit_state: EditSate,
}

#[derive(Default)]
struct EditSate {
    start: Option<String>,
    channels: Option<String>,
    addresses: Vec<Ipv4Addr>,
}

impl ExternalDeviceSettings {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(&mut self, ctx: &Context, project: &mut Option<Project>) {
        if !self.open {
            return;
        }
        let mut project = if let Some(project) = project {
            project
        } else {
            return;
        };

        self.edit_state.addresses = NetworkInterface::show()
            .expect("Could not find network interfaces")
            .into_iter()
            .flat_map(|interface| {
                interface
                    .addr
                    .into_iter()
                    .filter_map(|addr| match addr.ip() {
                        std::net::IpAddr::V4(ipv4) => Some(ipv4),
                        _ => None,
                    })
            })
            .collect::<Vec<_>>();
        self.edit_state.addresses.sort();

        ctx.show_viewport_immediate(
            ViewportId(Id::new("external devices settings window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(800.0, 500.0))
                .with_min_inner_size(Vec2::new(800.0, 500.0))
                .with_resizable(false)
                .with_minimize_button(false)
                .with_maximize_button(true),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "External Devices", |ui| {
                    let mut state = (&mut project, &mut self.edit_state);
                    let settings_menu = SettingsMenu::new(&mut state, &mut self.selected_submenu)
                        .add_submenu("Audio Input".into(), |ui, (project, edit_state)| {
                            audio_input_settings(ui, (**project, edit_state))
                        })
                        .add_submenu("Artnet Bridge".into(), |ui, (project, edit_state)| {
                            artnet_bridge_settings(ui, (**project, edit_state))
                        })
                        .add_submenu("Artnet Control".into(), |ui, (project, edit_state)| {
                            artnet_control_input_settings(ui, (**project, edit_state))
                        })
                        .add_submenu("Artnet Trigger".into(), |ui, (project, edit_state)| {
                            artnet_trigger_settings(ui, (**project, edit_state))
                        });
                    ui.add(settings_menu);
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}

fn audio_input_settings(ui: &mut Ui, state: (&mut Project, &mut EditSate)) {
    let (project, _edit_state) = state;
    ui.heading("Audio Input Device");
    let mut selected = project.audio_input_device.clone();
    let old_selected = selected.clone();

    let devices = AUDIO_DEVICES.lock().unwrap().clone();
    ui.selectable_value(&mut selected, None, "None");
    for (id, desc) in devices {
        ui.selectable_value(&mut selected, Some(id), desc.name());
    }

    if selected != old_selected {
        log::info!("Audio input device changed to {:?}", selected);
        {
            UiAction::SetAudioDevice(selected).enqueue();
        }
    }
}

fn artnet_control_input_settings(ui: &mut Ui, _state: (&mut Project, &mut EditSate)) {
    //let (project, edit_state) = state;
    ui.heading("Artnet Control");
}

fn artnet_bridge_settings(ui: &mut Ui, _state: (&mut Project, &mut EditSate)) {
    let mut config = ARTNET_CONFIG.lock();
    ui.heading("Artnet Bridge");
    ui.add_space(3.0);

    if config.bridge.is_empty() {
        ui.label("No artnet data received..");
    }

    egui::ScrollArea::vertical()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .id_salt("artnet_bridge_output_scroll")
        .show(ui, |ui| {
            ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                let mut hovered_output_routing = None;

                for (universe, bridge) in config.bridge.iter_mut() {
                    ui.label(RichText::new(format!("Universe: {universe}")).heading());

                    ui.horizontal(|ui| {
                        ui.label(format!("Updates: {}", bridge.updates));
                        ui.label(
                            bridge
                                .last_data_at
                                .with_timezone(&Local)
                                .format("(Last: %H:%M:%S)")
                                .to_string(),
                        );

                        bridge.output_routing.device.change_button(ui);

                        if let Some(device) = bridge.output_routing.device.and_then(Asset::get) {
                            let device_universes = device.data.universes();
                            if !device_universes.is_empty() {
                                ComboBox::new(format!("{universe}_universe"), "")
                                    .selected_text(match bridge.output_routing.universe {
                                        None => WidgetText::from("No universe selected"),
                                        Some(universe) => WidgetText::from(universe.to_string()),
                                    })
                                    .width(150.0)
                                    .show_ui(ui, |ui| {
                                        for device_universe in device_universes {
                                            let res = ui.selectable_value(
                                                &mut bridge.output_routing.universe,
                                                Some(*device_universe),
                                                device_universe.to_string(),
                                            );
                                            if res.hovered() {
                                                hovered_output_routing = Some(OutputRouting {
                                                    device: Some(device.id),
                                                    universe: Some(*device_universe),
                                                });
                                            }
                                        }
                                    });
                            }

                            ComboBox::new(format!("{universe}_merge"), "")
                                .selected_text(match bridge.htp {
                                    true => "HTP",
                                    false => "LTP",
                                })
                                .width(50.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut bridge.htp, true, "HTP");
                                    ui.selectable_value(&mut bridge.htp, false, "LTP");
                                });

                            if ui.add(Button::new(icons::X)).clicked() {
                                bridge.output_routing = OutputRouting::default();
                            }
                        }
                    });
                }

                *HOVERED_OUTPUT_ROUTING.lock() = hovered_output_routing;
            });
        });
}

fn artnet_trigger_settings(ui: &mut Ui, state: (&mut Project, &mut EditSate)) {
    let (_project, edit_state) = state;
    let mut config = ARTNET_CONFIG.lock();
    ui.heading("Artnet Trigger");
    ui.add_space(3.0);

    ui.horizontal(|ui| {
        ui.label("Active");
        ui.checkbox(&mut config.active, "");

        ui.label("Bind address");
        let mut selected_index = edit_state
            .addresses
            .iter()
            .position(|addr| addr == &config.bind_ip)
            .unwrap_or(0);
        egui::ComboBox::new("artnet_bind_address_combo", "")
            .selected_text(
                edit_state
                    .addresses
                    .get(selected_index)
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| "Unknown".to_string()),
            )
            .width(275.0)
            .show_ui(ui, |ui| {
                for (index, addr) in edit_state.addresses.iter().enumerate() {
                    ui.selectable_value(&mut selected_index, index, addr.to_string());
                }
            });
        if let Some(addr) = edit_state.addresses.get(selected_index) {
            config.bind_ip = *addr;
        }
    });
    ui.separator();
    ui.label("Universe");
    ui.add(Slider::new(&mut config.universe, 0..=32768));

    ui.label("Start channel");
    let start = edit_state
        .start
        .get_or_insert_with(|| config.start.to_string());
    if ui.add(TextEdit::singleline(start)).changed()
        && let Ok(start) = start.parse::<u16>()
    {
        config.start = start;
    }

    ui.label("Channels");
    let channels = edit_state
        .channels
        .get_or_insert_with(|| config.channels.to_string());
    if ui.add(TextEdit::singleline(channels)).changed()
        && let Ok(channels) = channels.parse::<u16>()
    {
        config.channels = channels;
    }
}

struct SettingsMenu<'a, S> {
    selected_submenu: &'a mut String,
    edit_state: &'a mut S,
    submenus: BTreeMap<String, Box<dyn FnOnce(&mut Ui, &mut S) + 'a>>,
}

impl<'a, S> SettingsMenu<'a, S> {
    fn new(edit_state: &'a mut S, selected_submenu: &'a mut String) -> Self {
        Self {
            selected_submenu,
            edit_state,
            submenus: BTreeMap::default(),
        }
    }
    fn add_submenu(
        mut self,
        submenu: String,
        ui_callback: impl FnOnce(&mut Ui, &mut S) + 'a,
    ) -> Self {
        self.submenus.insert(submenu, Box::new(ui_callback));
        self
    }
}

impl<'a, S> Widget for SettingsMenu<'a, S> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        debug_assert!(self.submenus.len() > 0);
        SidePanel::left("artnet_input_config")
            .resizable(true)
            .width_range(100.0..=200.0)
            .default_width(150.0)
            .show_inside(ui, |ui| {
                ui.take_available_space();
                self.submenus.keys().for_each(|key| {
                    ui.selectable_value(self.selected_submenu, key.clone(), key);
                });
            });
        if !self.submenus.contains_key(self.selected_submenu) {
            *self.selected_submenu = self.submenus.keys().next().unwrap().clone();
        }

        let submenu = self.submenus.remove(self.selected_submenu).unwrap();
        CentralPanel::default()
            .show_inside(ui, |ui| submenu(ui, self.edit_state))
            .response
    }
}
