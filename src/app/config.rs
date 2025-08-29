use crate::ui::{ChangeButton, action::UiAction};

use super::{App, timing::FadeMode};
use egui::{CentralPanel, Color32, Margin, RichText, TopBottomPanel};

impl App {
    pub fn config(&mut self, ui: &mut egui::Ui) {
        let Some(project) = self.project.as_mut() else {
            return;
        };

        TopBottomPanel::top("project config")
            .resizable(false)
            .frame(egui::Frame::NONE.inner_margin(Margin::from(4.0)))
            .show_inside(ui, |ui| {
                ui.scope(|ui| {
                    ui.set_max_width(ui.available_width());
                    ui.vertical_centered_justified(|ui| {
                        ui.add_space(2.0);
                        project.palette.change_button(ui);
                    });
                });

                if project.groups.is_empty() {
                    ui.painter().rect_filled(
                        {
                            let rect = ui.cursor();
                            rect.with_max_y(rect.min.y + 20.0)
                        },
                        0.0,
                        Color32::ORANGE,
                    );
                    ui.add_sized(
                        [ui.available_width(), 20.0],
                        egui::Label::new(
                            RichText::new("☢ No groups = no output! ☢").color(Color32::DARK_RED),
                        ),
                    );
                }
                ui.scope(|ui| {
                    ui.horizontal(|ui| {
                        if project.groups.change_button(ui) {
                            UiAction::InitGPU.enqueue();
                        }
                    });
                });
            });

        TopBottomPanel::bottom("Auto Mode")
            .resizable(false)
            .frame(egui::Frame::NONE.inner_margin(Margin::from(4.0)))
            .show_inside(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label("Fade duration:");
                    ui.radio_value(&mut self.timing.fade_mode, FadeMode::Instant, "Instant");
                    ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beat, "1 Beat");
                    ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beats4, "4 Beats");
                    ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beats16, "16 Beats");
                });
            });

        CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(Margin::from(4.0)))
            .show_inside(ui, |ui| {
                match project.scene_instance(self.selected_scene_instance) {
                    Some(scene_instance) => scene_instance.config_ui(ui),
                    None => {
                        ui.label("There's no Effect to configure.");
                    }
                };
            });
    }
}
