use super::{timing::FadeMode, App};
use crate::temperature::temperature;
use egui::{Color32, Context, Label, Layout, Margin, Stroke};

impl App {
    pub fn config(&mut self, ctx: &Context) {
        egui::SidePanel::left("left")
            .resizable(true)
            .default_width(280.0)
            .min_width(280.0)
            .max_width(ctx.used_rect().width() - 950.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);

                match self.project.scene_instance(self.selected_scene_instance) {
                    Some(scene_instance) => {
                        scene_instance.config_ui(ctx, ui);
                    }
                    None => {
                        ui.label("There's no Effect to configure.");
                    }
                }

                ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(6.0);

                    if let Some(framerate) = self.timing.framerate() {
                        ui.horizontal(|ui| {
                            ui.add(Label::new(format!("{framerate:.0} fps")));
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add(Label::new(temperature()))
                            });
                        });
                    }

                    ui.add_space(4.0);

                    egui::Frame::none()
                        .inner_margin(Margin::from(6.0))
                        .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.radio_value(
                                    &mut self.timing.fade_mode,
                                    FadeMode::Instant,
                                    "Instant",
                                );
                                ui.radio_value(
                                    &mut self.timing.fade_mode,
                                    FadeMode::Beat,
                                    "1 Beat",
                                );
                                ui.radio_value(
                                    &mut self.timing.fade_mode,
                                    FadeMode::Beats4,
                                    "4 Beats",
                                );
                                ui.radio_value(
                                    &mut self.timing.fade_mode,
                                    FadeMode::Beats16,
                                    "16 Beats",
                                );
                            });
                            ui.label("Fade Mode");
                        });
                });
            });
    }
}
