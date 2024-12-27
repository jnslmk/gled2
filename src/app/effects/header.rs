use super::App;
use crate::{
    storage::{
        asset::{project::scene_instance_path::SceneInstancePath, scene::Scene},
        asset_id::AssetId,
    },
    ui::{action, ChangeButton},
};
use egui::{Color32, RichText, Slider, Ui};

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
        if deck.groups.is_empty() {
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
            if deck.groups.change_button(ui) {
                action::UiAction::InitGPU.enqueue();
            }
        });
    }
}
