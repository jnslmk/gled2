use super::App;
use crate::scene::{Scene, SceneKind};
use egui::{Align, Checkbox, Layout, RichText, Slider, Ui};

impl App {
    pub fn scenes_header(&mut self, ui: &mut Ui, kind: SceneKind) {
        let scenes = match kind {
            SceneKind::Background => &mut self.persistant_state.background,
            SceneKind::Foreground => &mut self.persistant_state.foreground,
        };

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(match kind {
                    SceneKind::Background => "Background",
                    SceneKind::Foreground => "Foreground",
                })
                .heading(),
            );
            if ui.button("Add").clicked() {
                let mut scene = Scene::default();
                scene.kind = kind;
                self.selected_scene = self.pipeline.add_scene(scene);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        self.svg.is_some(),
                        Checkbox::new(&mut scenes.show_svg, RichText::new("SVG")),
                    )
                    .changed()
                {
                    self.persistant_state.dirty = true;
                };

                if ui
                    .checkbox(&mut scenes.always_render, RichText::new("Render all"))
                    .changed()
                {
                    self.persistant_state.dirty = true;
                };
                if ui
                    .add(
                        Slider::new(&mut scenes.size, 90.0..=500.0)
                            .show_value(false)
                            .text(RichText::new("Size")),
                    )
                    .changed()
                {
                    self.persistant_state.dirty = true;
                }
            });
        });
        ui.horizontal(|ui| {
            if kind == SceneKind::Background {
                ui.checkbox(&mut self.pipeline.background_auto_mode_active, "Auto Mode");
                ui.add_enabled(
                    self.pipeline.background_auto_mode_active,
                    Slider::new(&mut self.pipeline.background_auto_mode_seconds, 1..=240)
                        .custom_formatter(|n, _| format!("{} s", n)),
                );
                ui.label("Max Scenes:");
                ui.add_enabled(
                    self.pipeline.background_auto_mode_active,
                    Slider::new(&mut self.pipeline.background_auto_mode_max_scenes, 1..=10),
                );
            } else {
                ui.checkbox(&mut self.pipeline.foreground_auto_mode_active, "Auto Mode");
                ui.add_enabled(
                    self.pipeline.foreground_auto_mode_active,
                    Slider::new(&mut self.pipeline.foreground_auto_mode_seconds, 1..=240)
                        .custom_formatter(|n, _| format!("{} s", n)),
                );
            }
        });
    }
}
