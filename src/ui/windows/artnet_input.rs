use std::net::Ipv4Addr;

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
    Button, CentralPanel, ComboBox, Context, Id, Layout, RichText, SidePanel, Slider, TextEdit,
    Vec2, ViewportId, WidgetText,
};
use egui_phosphor_icons::icons;
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
                .with_inner_size(Vec2::new(800.0, 500.0))
                .with_min_inner_size(Vec2::new(800.0, 500.0))
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
                    let mut config = ARTNET_CONFIG.lock();

                    ui.horizontal(|ui| {
                        ui.label("Active");
                        ui.checkbox(&mut config.active, "");

                        ui.label("Bind address");
                        let mut selected_index = self
                            .addresses
                            .iter()
                            .position(|addr| addr == &config.bind_ip)
                            .unwrap_or(0);
                        egui::ComboBox::new("artnet_bind_address_combo", "")
                            .selected_text(
                                self.addresses
                                    .get(selected_index)
                                    .map(|addr| addr.to_string())
                                    .unwrap_or_else(|| "Unknown".to_string()),
                            )
                            .width(275.0)
                            .show_ui(ui, |ui| {
                                for (index, addr) in self.addresses.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut selected_index,
                                        index,
                                        addr.to_string(),
                                    );
                                }
                            });
                        if let Some(addr) = self.addresses.get(selected_index) {
                            config.bind_ip = *addr;
                        }
                    });

                    SidePanel::left("artnet_input_config")
                        .exact_width(ui.available_width() / 5.0)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            ui.heading("Input Config");
                            ui.add_space(3.0);
                            ui.label("Universe");
                            ui.add(Slider::new(&mut config.universe, 0..=32768));

                            ui.label("Start channel");
                            let start = self.start.get_or_insert_with(|| config.start.to_string());
                            if ui.add(TextEdit::singleline(start)).changed()
                                && let Ok(start) = start.parse::<u16>()
                            {
                                config.start = start;
                            }

                            ui.label("Channels");
                            let channels = self
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
                                        let mut hovered_output_routing = None;

                                        for (universe, bridge) in config.bridge.iter_mut() {
                                            ui.label(
                                                RichText::new(format!("Universe: {universe}"))
                                                    .heading(),
                                            );

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

                                                if let Some(device) = bridge
                                                    .output_routing
                                                    .device
                                                    .and_then(Asset::get)
                                                {
                                                    let device_universes = device.data.universes();
                                                    if !device_universes.is_empty() {
                                                        ComboBox::new(
                                                            format!("{universe}_universe"),
                                                            "",
                                                        )
                                                        .selected_text(
                                                            match bridge.output_routing.universe {
                                                                None => WidgetText::from(
                                                                    "No universe selected",
                                                                ),
                                                                Some(universe) => WidgetText::from(
                                                                    universe.to_string(),
                                                                ),
                                                            },
                                                        )
                                                        .width(150.0)
                                                        .show_ui(ui, |ui| {
                                                            for device_universe in device_universes
                                                            {
                                                                let res = ui.selectable_value(
                                                                    &mut bridge
                                                                        .output_routing
                                                                        .universe,
                                                                    Some(*device_universe),
                                                                    device_universe.to_string(),
                                                                );
                                                                if res.hovered() {
                                                                    hovered_output_routing =
                                                                        Some(OutputRouting {
                                                                            device: Some(device.id),
                                                                            universe: Some(
                                                                                *device_universe,
                                                                            ),
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
                                                            ui.selectable_value(
                                                                &mut bridge.htp,
                                                                true,
                                                                "HTP",
                                                            );
                                                            ui.selectable_value(
                                                                &mut bridge.htp,
                                                                false,
                                                                "LTP",
                                                            );
                                                        });

                                                    if ui.add(Button::new(icons::X)).clicked() {
                                                        bridge.output_routing =
                                                            OutputRouting::default();
                                                    }
                                                }
                                            });
                                        }

                                        *HOVERED_OUTPUT_ROUTING.lock() = hovered_output_routing;
                                    },
                                );
                            });
                    });
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
