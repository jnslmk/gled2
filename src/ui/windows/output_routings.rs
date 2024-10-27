use crate::{app::svg::universes, extract_output::ExtractOutput};
use egui::{ComboBox, Context, Layout, RichText};

#[derive(Default)]
pub struct OutputRoutingsWindow {
    open: bool,
}

impl OutputRoutingsWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        egui::Window::new("Config Output Routings")
            .collapsible(false)
            .resizable(true)
            .min_width(300.0)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("output_scroll")
                    .show(ui, |ui| {
                        let extract_output = ExtractOutput::get();
                        let devices = extract_output.devices.lock();
                        let mut routings = extract_output.routings.lock();

                        ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                            let universes = universes();
                            extract_output
                                .universes
                                .lock()
                                .retain(|universe| universes.contains(universe));

                            if universes.is_empty() {
                                ui.label(RichText::new("No universes available. You need to load a svg file first!").heading());
                            }

                            for universe in universes {
                                ui.label(RichText::new(format!("Universe: {universe}")).heading());

                                ui.horizontal(|ui| {
                                    let output_routing = routings.universe_output_routing(universe);
                                    let device = output_routing
                                        .device
                                        .as_ref()
                                        .map(|device| devices.get(device));

                                    ComboBox::new(format!("{universe}_device"), "")
                                        .selected_text(match (devices.is_empty(), device) {
                                            (true, _) => "No devices configured",
                                            (false, None) => "No device selected",
                                            (false, Some(None)) => "Device not found",
                                            (false, Some(Some(device))) => device.name(),
                                        })
                                        .width(150.0)
                                        .show_ui(ui, |ui| {
                                            for (id, device) in devices.iter() {
                                                if ui
                                                    .selectable_value(
                                                        &mut output_routing.device.unwrap_or_default(),
                                                        *id,
                                                        device.name(),
                                                    )
                                                    .changed()
                                                {
                                                    output_routing.device = Some(*id);
                                                    output_routing.universe.take();
                                                }
                                            }
                                        });

                                    if let Some(device) = device.flatten() {
                                        if !device.universes().is_empty() {
                                            ComboBox::new(format!("{universe}_universe"), "")
                                                .selected_text(match output_routing.universe {
                                                    None => "No universe selected".to_string(),
                                                    Some(universe) => universe.to_string(),
                                                })
                                                .width(150.0)
                                                .show_ui(ui, |ui| {
                                                    for universe in device.universes() {
                                                        if ui
                                                            .selectable_value(
                                                                &mut output_routing
                                                                    .universe
                                                                    .unwrap_or_default(),
                                                                *universe,
                                                                universe.to_string(),
                                                            )
                                                            .changed()
                                                        {
                                                            output_routing.universe = Some(*universe);
                                                        }
                                                    }
                                                });
                                        }
                                    }
                                });

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
