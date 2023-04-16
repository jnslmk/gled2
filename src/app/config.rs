mod color;
pub mod group;

use super::App;
use crate::animation::Color;
use egui::{Button, Checkbox, Color32, Context, Layout, Modifiers, RichText, Slider};
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

                let mut delete_scene = false;
                match pipeline.scene(self.selected_scene) {
                    Some(scene) => {
                        ui.label(RichText::new("Colors").heading());
                        color::selection(ui, &mut scene.palette, all_colors);

                        ui.separator();

                        ui.label(RichText::new("Settings").heading());
                        ui.add(Checkbox::new(&mut scene.artnet_extraction, "Active"));
                        ui.add(
                            Slider::new(&mut scene.opacity, 0.0..=1.0)
                                .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0))
                                .text("Opacity"),
                        );
                        ui.add(
                            Slider::new(&mut scene.beat_progression_offset, 0.0..=1.0)
                                .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0))
                                .text("Beat offset"),
                        );
                        scene.config_ui(ui);

                        ui.separator();

                        ui.label(RichText::new("Group").heading());
                        group::selection(ui, &mut scene.group);

                        ui.separator();

                        ui.vertical_centered_justified(|ui| {
                            ui.style_mut().spacing.interact_size.y = 40.0;

                            if ui
                                .add(
                                    Button::new("🗑 Remove Scene")
                                        .fill(Color32::DARK_RED)
                                        .shortcut_text("Del"),
                                )
                                .clicked()
                                || ctx.input_mut(|i| {
                                    i.consume_key(Modifiers::default(), egui::Key::Backspace)
                                        || i.consume_key(Modifiers::default(), egui::Key::Delete)
                                })
                            {
                                delete_scene = true;
                            }
                        });

                        if let Some(framerate) = self
                            .timing
                            .framerate()
                            .filter(|_| self.persistant_state.fullscreen)
                        {
                            ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
                                ui.label(format!("{framerate:.01} fps"));
                            });
                        }
                    }
                    None => {
                        ui.label("There's no Scene to configure.");
                    }
                }

                if delete_scene {
                    let kind = pipeline
                        .remove_scene(self.selected_scene)
                        .map(|scene| scene.kind);

                    self.selected_scene = pipeline
                        .scenes()
                        .into_iter()
                        .find(|(_index, scene)| match kind {
                            Some(kind) => scene.kind == kind,
                            None => true,
                        })
                        .map(|(index, _scene)| index)
                        .unwrap_or_default();
                }
            });
    }
}
