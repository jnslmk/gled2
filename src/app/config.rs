use super::{timing::FadeMode, App};
use egui::{Context, Layout, Margin, Slider, ViewportId};

impl App {
    pub fn config(&mut self, ctx: &Context, viewport_id: Option<ViewportId>) {
        let Some(project) = self.project.as_mut() else {
            return;
        };

        egui::TopBottomPanel::bottom(format!("{viewport_id:?} config"))
            .min_height(300.0)
            .resizable(true)
            .show(ctx, |ui| {
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
                    ui.horizontal(|ui| {
                        ui.spacing_mut().slider_width = ui.available_width() - 30.0;
                        ui.spacing_mut().slider_rail_height = 10.0;
                        ui.label("A");
                        ui.add(
                            Slider::new(&mut project.cross_fader, 0.0..=1.0)
                                .handle_shape(egui::style::HandleShape::Rect { aspect_ratio: 3.0 })
                                .show_value(false),
                        );
                        ui.label("B");
                    });

                    egui::Frame::none()
                        .inner_margin(Margin::from(6.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label("Fade Mode:");
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
                        });
                });
            });
    }
}
