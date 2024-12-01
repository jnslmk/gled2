use super::App;
use crate::{
    storage::{AssetId, Scene, SceneInstancePath},
    ui::{action, ChangeButton},
};
use egui::{RichText, Slider, Ui};

impl App {
    pub fn effects_header(&mut self, ui: &mut Ui, path: SceneInstancePath) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let deck = project.deck(path);

        ui.horizontal(|ui| {
            ui.label(RichText::new(path.deck_path.name()).heading());

            let mut scene: Option<AssetId<Scene>> = None;
            scene.change_button(ui);
            if let Some(scene) = scene {
                deck.add_scene(&mut self.selected_scene_instance, scene);
            }

            ui.menu_button(
                if deck.auto_mode_active {
                    "💂Automatic"
                } else {
                    "🔨Manual"
                },
                |ui| {
                    ui.label("Automatic mode");
                    ui.checkbox(&mut deck.auto_mode_active, "");
                    ui.label("Seconds/scene");
                    ui.add_enabled(
                        deck.auto_mode_active,
                        Slider::new(&mut deck.auto_mode_seconds, 1..=240)
                            .custom_formatter(|n, _| format!("{} s", n)),
                    );
                    ui.label("Max concurrent scenes");
                    ui.add_enabled(
                        deck.auto_mode_active,
                        Slider::new(&mut deck.auto_mode_max_scenes, 1..=10),
                    );
                },
            );

            ui.scope(|ui| {
                ui.vertical_centered_justified(|ui| {
                    deck.palette.change_button(ui);
                });
            });
        });
        ui.horizontal(|ui| {
            if deck.groups.change_button(ui) {
                action::Action::InitGPU.enqueue();
            }
        });
    }
}
