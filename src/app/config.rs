mod color;
pub mod group;

use super::App;
use crate::{animation::Color, scene::Scene};
use egui::{Context, Layout, RichText, Ui};
use std::collections::BTreeSet;

impl App {
    pub fn config(&mut self, ctx: &Context) {
        egui::SidePanel::left("left")
            .resizable(false)
            .default_width(200.0)
            .show(ctx, |ui| {
                crate::get_pipeline!(pipeline);
                let all_colors: BTreeSet<Color> = std::iter::once(Color::default())
                    .chain(
                        pipeline
                            .scenes()
                            .into_iter()
                            .flat_map(|(_index, scene)| scene.palette.colors.iter().copied()),
                    )
                    .collect();

                match pipeline.scenes().get_mut(self.selected_scene) {
                    Some((_index, scene)) => {
                        self.scene_config(ui, scene, all_colors);
                    }
                    None => {
                        ui.label("There's no Scene to configure.");
                    }
                }
            });
    }

    fn scene_config(&mut self, ui: &mut Ui, scene: &mut Scene, all_colors: BTreeSet<Color>) {
        ui.label(RichText::new("Colors").heading());
        color::selection(ui, &mut scene.palette, all_colors);

        ui.separator();

        ui.label(RichText::new("Settings").heading());
        ui.label("TODO: Animation settings");

        ui.separator();

        ui.label(RichText::new("Group").heading());
        group::selection(ui, scene.group_mut());

        ui.separator();

        if let Some(framerate) = self.timing.framerate().filter(|_| self.fullscreen) {
            ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.label(format!("{framerate:.01} fps"));
            });
        }
    }
}
