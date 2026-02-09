use std::net::Ipv4Addr;

use crate::{
    input::artnet::ARTNET_CONFIG,
    ui::window_common::{default_viewport_builder, gled_window_frame},
};
use chrono::Local;
use egui::{CentralPanel, Context, Id, Layout, Response, RichText, SidePanel, Slider, TextEdit, Ui, Vec2, ViewportId, Widget};
use egui::ahash::HashMap;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};

#[derive(Default)]
pub struct ArtnetInputWindow {
    open: bool,
    start: Option<String>,
    channels: Option<String>,
    addresses: Vec<Ipv4Addr>,
}

impl ArtnetInputWindow {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        self.addresses = NetworkInterface::show()
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
        self.addresses.sort();

        ctx.show_viewport_immediate(
            ViewportId(Id::new("artnet inputs window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(800.0, 400.0))
                .with_min_inner_size(Vec2::new(800.0, 400.0))
                .with_resizable(false)
                .with_minimize_button(false)
                .with_maximize_button(false),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "Artnet Inputs/Bridge", |ui| {
                    let settings_menu = SettingsMenu::new(self)
                        .add_submenu("Artnet Bridge".into(), |ui, edit_state|
                            artnet_bridge_settings(ui, edit_state));
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

fn artnet_bridge_settings(ui: &mut Ui, edit_state: &mut ArtnetInputWindow, ) -> Response {
    let mut config = ARTNET_CONFIG.lock();

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
                edit_state.addresses
                    .get(selected_index)
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| "Unknown".to_string()),
            )
            .width(275.0)
            .show_ui(ui, |ui| {
                for (index, addr) in edit_state.addresses.iter().enumerate() {
                    ui.selectable_value(
                        &mut selected_index,
                        index,
                        addr.to_string(),
                    );
                }
            });
        if let Some(addr) = edit_state.addresses.get(selected_index) {
            config.bind_ip = *addr;
        }
    });

    SidePanel::left("artnet_input_config")
        .exact_width(ui.available_width() / 3.0)
        .resizable(false)
        .show_inside(ui, |ui| {
            ui.heading("Input Config");
            ui.add_space(3.0);
            ui.label("Universe");
            ui.add(Slider::new(&mut config.universe, 0..=32768));

            ui.label("Start channel");
            let start = edit_state.start.get_or_insert_with(|| config.start.to_string());
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
        });

    CentralPanel::default().show_inside(ui, |ui| {
        ui.heading("Artnet Bridge");
        ui.add_space(3.0);

        if config.bridge.is_empty() {
            ui.label("No artnet data received..");
        }

        egui::ScrollArea::vertical()
            .scroll_bar_visibility(
                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
            )
            .id_salt("artnet_bridge_output_scroll")
            .show(ui, |ui| {
                ui.with_layout(
                    Layout::top_down_justified(egui::Align::Min),
                    |ui| {
                        for (universe, bridge) in config.bridge.iter_mut() {
                            ui.label(
                                RichText::new(format!("Universe: {universe}"))
                                    .heading(),
                            );

                            ui.horizontal(|ui| {
                                ui.label(
                                    bridge
                                        .last_data_at
                                        .with_timezone(&Local)
                                        .format("Last data at: %H:%M:%S")
                                        .to_string(),
                                );

                                let mut active = bridge.output_universe.is_some();
                                if ui.checkbox(&mut active, "Enable").changed() {
                                    if active {
                                        bridge.output_universe = Some(*universe);
                                    } else {
                                        bridge.output_universe.take();
                                    }
                                }
                                if let Some(universe) =
                                    bridge.output_universe.as_mut()
                                {
                                    ui.label("Output universe:");
                                    ui.add(Slider::new(universe, 0..=32768));
                                }
                            });
                        }
                    },
                );
            });
    });
    ui.response()
}

fn artnet_input_settings(edit_state: &mut ArtnetInputWindow, ui: &mut Ui){

}

struct SettingsMenu<'a, S>{
    edit_state: &'a mut S,
    submenus: HashMap<String, Box<dyn Fn(&mut Ui, &mut S) -> Response + 'a>>,
}

impl<'a, S> SettingsMenu<'a, S> {
    fn new(edit_state: &'a mut S,) -> Self{
        Self{
            edit_state,
            submenus: HashMap::default(),
        }
    }
    fn add_submenu(mut self, submenu: String, ui_callback: impl Fn(&mut Ui, &mut S) -> Response + 'a) -> Self {
        self.submenus.insert(submenu, Box::new(ui_callback));
        self
    }
}

impl <'a, S> Widget for SettingsMenu<'a, S> {
    fn ui(self, ui: &mut Ui) -> Response {
        debug_assert!(self.submenus.len() > 0);
        let mut selected = self.submenus.keys().next().unwrap().to_string();
        self.submenus.keys().for_each(|key| {
            ui.selectable_value(&mut selected, key.to_string(), key);
        });
        ui.separator();
        let submenu = self.submenus.get(&selected)
            .expect(format!("Submenu {} could not be found", selected).as_str());
        submenu(ui, self.edit_state)
    }
}