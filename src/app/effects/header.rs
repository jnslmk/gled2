use super::App;
use egui::{Slider, Ui};

impl App {
    pub fn effects_header(&mut self, ui: &mut Ui) {
        let Some(project) = self.project.as_mut() else {
            return;
        };

        ui.horizontal(|ui| {
            ui.menu_button(
                if project.auto_mode_active {
                    "💂Automatic"
                } else {
                    "🔨Manual"
                },
                |ui| {
                    ui.label("Automatic mode");
                    ui.checkbox(&mut project.auto_mode_active, "");
                    ui.label("Seconds/scene");
                    ui.add_enabled(
                        project.auto_mode_active,
                        Slider::new(&mut project.auto_mode_seconds, 1..=240)
                            .custom_formatter(|n, _| format!("{n} s")),
                    );
                    ui.label("Max concurrent scenes");
                    ui.add_enabled(
                        project.auto_mode_active,
                        Slider::new(&mut project.auto_mode_max_scenes, 1..=10),
                    );
                },
            );
        });
    }
}
