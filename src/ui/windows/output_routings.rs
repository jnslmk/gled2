use crate::{
    app::svg::Svg,
    pipeline::extract_output::ExtractOutput,
    storage::asset::{Asset, output_device::routing::OutputRouting},
    ui::{
        ChangeButton,
        window_common::{default_viewport_builder, gled_window_frame},
    },
};
use egui::{
    Color32, ComboBox, Context, Id, Layout, RichText, Vec2, ViewportId, WidgetText, mutex::Mutex,
};
use once_cell::sync::Lazy;

pub static HOVERED_OUTPUT_ROUTING: Lazy<Mutex<Option<OutputRouting>>> =
    Lazy::new(|| Mutex::new(None));

#[derive(Default)]
pub struct OutputRoutingsWindow {
    open: bool,
}

impl OutputRoutingsWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        let mut hovered_output_routing = None;
        ctx.show_viewport_immediate(
            ViewportId(Id::new("output routings window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(340.0, 500.0))
                .with_min_inner_size(Vec2::new(340.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "Output Routings", |ui| {
                    let extract_output = ExtractOutput::get();
                    let mut routings = extract_output.routings.lock();
                    let used_multiple_times = routings.output_universes_which_are_used_multiple_times();
                    if !used_multiple_times.is_empty() {
                        ui.colored_label(
                            Color32::RED,
                            RichText::new("Some output universes are used multiple times! This can lead to unexpected results.")
                                .heading(),
                        );
                    }

                    egui::ScrollArea::vertical()
                        .id_salt("output_scroll")
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                                let universes = Svg::universes();
                                routings.remove_old(&universes);

                                extract_output
                                    .universes
                                    .lock()
                                    .retain(|universe| universes.contains(universe));

                                if universes.is_empty() {
                                    ui.label(RichText::new(
                                    "No universes available. You need to load a svg file first!",
                                ));
                                }

                                let mut set_routing = None;
                                let mut set_device = None;

                                for universe in universes.clone() {
                                    ui.label(
                                        RichText::new(format!("Universe: {universe}")).heading(),
                                    );

                                    ui.horizontal(|ui| {
                                        let output_routing =
                                            routings.universe_output_routing(universe);
                                        if output_routing.device.change_button(ui) {
                                            set_device = Some((universe, output_routing.device));
                                        }

                                        if let Some(device) =
                                            output_routing.device.and_then(Asset::get)
                                        {
                                            let device_universes = device.data.universes();
                                            if !device_universes.is_empty() {
                                                ComboBox::new(format!("{universe}_universe"), "")
                                                    .selected_text(match output_routing.universe {
                                                        None => WidgetText::from("No universe selected"),
                                                        Some(universe) => {
                                                            let text = WidgetText::from(universe.to_string());
                                                            if used_multiple_times.contains(&(device.id, universe)) {
                                                                text.color(Color32::RED)
                                                            } else {
                                                                text
                                                            }
                                                        }
                                                    })
                                                    .width(150.0)
                                                    .show_ui(ui, |ui| {
                                                        for device_universe in device_universes {
                                                            let res = ui
                                                                .selectable_value(
                                                                    &mut output_routing
                                                                        .universe,
                                                                    Some(*device_universe),
                                                                    device_universe.to_string(),
                                                                );
                                                            if res.hovered() {
                                                                hovered_output_routing = Some(OutputRouting { device: Some(device.id), universe: Some(*device_universe) });
                                                            }
                                                            if res
                                                                .changed()
                                                            {
                                                                set_routing = Some((
                                                                    universe,
                                                                    *device_universe,
                                                                ));
                                                            }
                                                        }
                                                    });
                                            }
                                        }
                                    });

                                    ui.separator();
                                }

                                if let Some((mut universe, device)) = set_device {
                                    let mut first = true;
                                    loop {
                                        let output_routing =
                                            routings.universe_output_routing(universe);
                                        if first || output_routing.device.is_none() {
                                            output_routing.device = device;

                                            if let Some(device) = device.and_then(Asset::get) {
                                                let device_universes = device.data.universes();
                                                if device_universes.len() == 1 {
                                                    output_routing.universe =
                                                        Some(device_universes[0]);
                                                    break;
                                                }
                                            };
                                        } else {
                                            break;
                                        }

                                        first = false;
                                        universe += 1;
                                        if !universes.contains(&universe) {
                                            break;
                                        }
                                    }
                                }
                                if let Some((mut universe, mut device_universe)) = set_routing {
                                    let mut first = true;
                                    loop {
                                        let output_routing =
                                            routings.universe_output_routing(universe);
                                        let Some(device) =
                                            output_routing.device.and_then(Asset::get)
                                        else {
                                            break;
                                        };
                                        if !device.data.universes().contains(&device_universe) {
                                            break;
                                        }
                                        if first || output_routing.universe.is_none() {
                                            output_routing.universe = Some(device_universe);
                                        } else {
                                            break;
                                        }

                                        first = false;
                                        universe += 1;
                                        device_universe += 1;
                                        if !universes.contains(&universe) {
                                            break;
                                        }
                                    }
                                }
                            });
                        });
                });
            },
        );

        *HOVERED_OUTPUT_ROUTING.lock() = hovered_output_routing;
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}
