use super::{App, timing::FadeMode};
use egui::{Layout, Margin, Slider};
use egui_tiles::UiResponse;

impl App {
    pub fn config(&mut self, ui: &mut egui::Ui) -> UiResponse {
        let Some(project) = self.project.as_mut() else {
            return UiResponse::None;
        };

        let res = match project.scene_instance(self.selected_scene_instance) {
            Some(scene_instance) => scene_instance.config_ui(ui),
            None => {
                ui.label("There's no Effect to configure.");
                UiResponse::None
            }
        };

        ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.horizontal(|ui| {
                if ui.button("⏮").clicked() {
                    project.cross_fader = 0.0;
                }
                if ui.button("⏸").clicked() {
                    project.cross_fader = 0.5;
                }
                if ui.button("⏭").clicked() {
                    project.cross_fader = 1.0;
                }

                ui.spacing_mut().slider_width = ui.available_width() - 32.0;
                ui.spacing_mut().slider_rail_height = 10.0;

                ui.label("A");
                ui.add(
                    Slider::new(&mut project.cross_fader, 0.0..=1.0)
                        .handle_shape(egui::style::HandleShape::Rect { aspect_ratio: 3.0 })
                        .show_value(false),
                );
                ui.label("B");
            });

            egui::Frame::NONE
                .inner_margin(Margin::from(6.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label("Fade duration:");
                        ui.radio_value(&mut self.timing.fade_mode, FadeMode::Instant, "Instant");
                        ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beat, "1 Beat");
                        ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beats4, "4 Beats");
                        ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beats16, "16 Beats");
                    });
                });

            res
        })
        .inner
    }
}
