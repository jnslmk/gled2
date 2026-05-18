use super::App;
use crate::ui::ChangeButton;
use egui::{CentralPanel, Color32, Margin, RichText};

impl App {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn config(&mut self, ui: &mut egui::Ui) {
        let svg = self.svg_texture(ui.ctx());
        let Some(project) = self.project.as_mut() else {
            return;
        };

        egui::Panel::top("project config")
            .resizable(false)
            .frame(egui::Frame::NONE.inner_margin(Margin::from(4.0)))
            .show_inside(ui, |ui| {
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
                    project.groups.change_button(ui);
                });
            });

        CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(Margin::from(4.0)))
            .show_inside(ui, |ui| {
                let groups = project.groups.clone();
                match project.get_scenes_instance(&self.selected_scene_instance) {
                    Some(scene_instance) => scene_instance.config_ui(
                        ui,
                        &mut self.selected_scene_effect_editor,
                        svg,
                        scene_instance.groups_overwrite.clone().unwrap_or(groups),
                        &self.timing,
                        &mut self.collections,
                        &mut self.sound_data,
                    ),
                    None => {
                        ui.label("There's no Effect to configure.");
                    }
                };
            });
    }
}
