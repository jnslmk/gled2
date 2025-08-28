use super::App;
use crate::{
    storage::{
        asset::{project::DeckPath, scene::Scene},
        asset_id::AssetId,
    },
    ui::ChangeButton,
};
use egui::{Slider, Ui};

impl App {
    pub fn effects_header(&mut self, ui: &mut Ui, deck_path: DeckPath) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let mut init_gpu = false;

        ui.horizontal(|ui| {
            let mut scene: Option<AssetId<Scene>> = None;
            scene.change_button(ui);
            if let Some(scene) = scene {
                self.selected_scene_instance = project.add_scene(deck_path, scene);
                init_gpu = true;
            }

            if deck_path == DeckPath::Grid {
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
            }
        });

        if init_gpu {
            project.init_gpu();
        }
    }
}
