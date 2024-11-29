use super::{timing::FadeMode, App};
use egui::{Color32, Context, Layout, Margin, Stroke};

impl App {
    pub fn config(&mut self, ctx: &Context) {
        let Some(project) = self.project.as_mut() else {
            return;
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(6.0);

            match project.scene_instance(self.selected_scene_instance) {
                Some(scene_instance) => {
                    scene_instance.config_ui(ctx, ui);
                }
                None => {
                    ui.label("There's no Effect to configure.");
                }
            }

            ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(6.0);

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
                            ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beat, "1 Beat");
                            ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beats4, "4 Beats");
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
