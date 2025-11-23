use crate::ui::{ChangeButton, action::UiAction};

use super::{App, timing::FadeMode};
use egui::{CentralPanel, Color32, Margin, RichText, TopBottomPanel};

impl App {
    pub fn config(&mut self, ui: &mut egui::Ui) {
        let svg = self.svg_mut().and_then(|svg| svg.image(ui.ctx()));
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

                ui.add_space(4.0);

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
                ui.horizontal(|ui| {
                    if project.groups.change_button(ui) {
                        UiAction::InitGPU.enqueue();
                    }
                });

                ui.add_space(4.0);

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
                let groups = project.groups.clone();
                match project.scene_instance_mut(self.selected_scene_instance) {
                    Some(scene_instance) => scene_instance.config_ui(
                        ui,
                        svg,
                        scene_instance.groups_overwrite.clone().unwrap_or(groups),
                        &self.timing,
                    ),
                    None => {
                        ui.label("There's no Effect to configure.");
                    }
                };
            });
    }
}
