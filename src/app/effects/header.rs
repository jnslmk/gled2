use super::App;
use crate::{
    app::PersistantState,
    storage::{AssetId, Scene, SceneInstancePath},
    ui::{action, ChangeButton},
};
use egui::{Align, Checkbox, Layout, RichText, Slider, Ui};

impl App {
    pub fn effects_header(&mut self, ui: &mut Ui) {
        let mut effects_show_svg = PersistantState::effects_show_svg();
        let mut effects_always_render = PersistantState::effects_always_render();
        let mut effects_size = PersistantState::effects_size();

        let has_svg = self.svg().is_some();
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let deck = project.deck(SceneInstancePath::default());

        ui.horizontal(|ui| {
            ui.label(RichText::new("Scenes").heading());

            let mut scene: Option<AssetId<Scene>> = None;
            scene.change_button(ui);
            if let Some(scene) = scene {
                deck.add_scene(&mut self.selected_scene_instance, scene);
            }

            if deck.groups.change_button(ui) {
                action::Action::InitGPU.enqueue();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        has_svg,
                        Checkbox::new(&mut effects_show_svg, RichText::new("SVG")),
                    )
                    .changed()
                {
                    let mut persistant_state = PersistantState::get();
                    persistant_state.effects_show_svg = effects_show_svg;
                    persistant_state.save();
                };

                if ui
                    .checkbox(&mut effects_always_render, RichText::new("Render all"))
                    .changed()
                {
                    let mut persistant_state = PersistantState::get();
                    persistant_state.effects_always_render = effects_always_render;
                    persistant_state.save();
                };
                if ui
                    .add(
                        Slider::new(&mut effects_size, 90.0..=500.0)
                            .show_value(false)
                            .text(RichText::new("Size")),
                    )
                    .changed()
                {
                    let mut persistant_state = PersistantState::get();
                    persistant_state.effects_size = effects_size;
                    persistant_state.save();
                }
            });
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut deck.auto_mode_active, "Auto Mode");
            ui.add_enabled(
                deck.auto_mode_active,
                Slider::new(&mut deck.auto_mode_seconds, 1..=240)
                    .custom_formatter(|n, _| format!("{} s", n)),
            );
            ui.label("Max Effects:");
            ui.add_enabled(
                deck.auto_mode_active,
                Slider::new(&mut deck.auto_mode_max_scenes, 1..=10),
            );
            ui.vertical_centered_justified(|ui| {
                deck.palette.change_button(ui);
            });
        });
    }
}
