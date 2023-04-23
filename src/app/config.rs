mod color;
pub mod group;

use super::{timing::FadeMode, App};
use crate::{
    animation::{Animation, AnimationConfig, Color},
    hotkey::Hotkey,
};
use egui::{Button, Checkbox, Color32, Context, Layout, Modifiers, RichText, Slider};
use std::collections::BTreeSet;
use strum::IntoEnumIterator;

impl App {
    pub fn config(&mut self, ctx: &Context) {
        egui::SidePanel::left("left")
            .resizable(true)
            .default_width(280.0)
            .min_width(280.0)
            .max_width(ctx.used_rect().width() - 950.0)
            .show(ctx, |ui| {
                let all_colors: BTreeSet<Color> = std::iter::once(Color::default())
                    .chain(
                        self.pipeline
                            .scenes()
                            .into_iter()
                            .flat_map(|(_index, scene)| scene.palette.colors.iter().copied()),
                    )
                    .collect();

                let svg = self
                    .svg
                    .as_mut()
                    .filter(|_| self.persistant_state.show_preview_svg)
                    .and_then(|svg| svg.image(&mut self.pipeline))
                    .map(|svg| svg.texture_id(ctx));
                let mut action = Action::None;
                match self.pipeline.scene(self.selected_scene) {
                    Some(scene) => {
                        ui.label(RichText::new("Hotkey").heading());
                        ui.vertical_centered_justified(|ui| {
                            ui.style_mut().spacing.interact_size.y = 40.0;
                            ui.menu_button(
                                match scene.hotkey {
                                    Some(hotkey) => format!("{hotkey}"),
                                    None => "Assign".to_string(),
                                },
                                |ui| {
                                    ui.label("Please press a key!");
                                    if let Some(hotkey) = Hotkey::get(ctx, &self.gamepad) {
                                        scene.hotkey = Some(hotkey);
                                        ui.close_menu();
                                    }
                                },
                            );
                        });

                        ui.separator();

                        ui.label(RichText::new("Flash Hotkey").heading());
                        ui.vertical_centered_justified(|ui| {
                            ui.style_mut().spacing.interact_size.y = 40.0;
                            ui.menu_button(
                                match scene.flash_hotkey {
                                    Some(hotkey) => format!("{hotkey}"),
                                    None => "Assign".to_string(),
                                },
                                |ui| {
                                    ui.label("Please press a key!");
                                    if let Some(hotkey) = Hotkey::get(ctx, &self.gamepad) {
                                        scene.flash_hotkey = Some(hotkey);
                                        ui.close_menu();
                                    }
                                },
                            );
                        });
                        ui.separator();

                        ui.label(RichText::new("Colors").heading());
                        color::color_selection(
                            ui,
                            &mut scene.palette,
                            all_colors,
                            scene.animation.uses_multiple_colors(),
                        );

                        ui.separator();

                        ui.label(RichText::new("Animation").heading());
                        let animation_changed = egui::ComboBox::from_label("Animation")
                            .selected_text(format!("{}", scene.animation))
                            .show_ui(ui, |ui| {
                                let mut changed = false;
                                for animation in Animation::iter() {
                                    let text = format!("{animation}");
                                    if ui
                                        .selectable_value(&mut scene.animation, animation, text)
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                }
                                changed
                            })
                            .inner
                            .unwrap_or_default();
                        if animation_changed {
                            scene.reset_gpu_state();
                            action = Action::InitGpu;
                        }

                        ui.add(Checkbox::new(&mut scene.active, "Active"));
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

                        scene.config_ui(ui, svg);

                        ui.separator();

                        ui.label(RichText::new("Group").heading());
                        group::selection(ui, &mut scene.group);

                        ui.separator();

                        ui.vertical_centered_justified(|ui| {
                            ui.style_mut().spacing.interact_size.y = 40.0;

                            if ui
                                .add(Button::new("🗐 Duplicate Scene").fill(Color32::DARK_BLUE))
                                .clicked()
                            {
                                action = Action::Clone;
                            }
                        });
                        ui.vertical_centered_justified(|ui| {
                            ui.style_mut().spacing.interact_size.y = 40.0;

                            if ui
                                .add(
                                    Button::new(match scene.kind {
                                        crate::scene::SceneKind::Background => {
                                            "➡ Move to Foreground"
                                        }
                                        crate::scene::SceneKind::Foreground => {
                                            "⬅ Move to Background"
                                        }
                                    })
                                    .fill(Color32::from_rgb(204, 85, 0)),
                                )
                                .clicked()
                            {
                                action = Action::Move;
                            }
                        });
                        ui.vertical_centered_justified(|ui| {
                            ui.style_mut().spacing.interact_size.y = 40.0;

                            if ui
                                .add(
                                    Button::new("🗑 Remove Scene")
                                        .fill(Color32::DARK_RED)
                                        .shortcut_text("Del"),
                                )
                                .clicked()
                                || {
                                    !ctx.wants_keyboard_input()
                                        && ctx.input_mut(|i| {
                                            i.consume_key(
                                                Modifiers::default(),
                                                egui::Key::Backspace,
                                            ) || i.consume_key(
                                                Modifiers::default(),
                                                egui::Key::Delete,
                                            )
                                        })
                                }
                            {
                                action = Action::Delete;
                            }
                        });

                        ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
                            if let Some(framerate) = self.timing.framerate() {
                                ui.label(format!("{framerate:.01} fps"));
                            }

                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.radio_value(
                                    &mut self.timing.fade_mode,
                                    FadeMode::Instant,
                                    "Instant",
                                );
                                ui.radio_value(
                                    &mut self.timing.fade_mode,
                                    FadeMode::Beat,
                                    "1 Beat",
                                );
                                ui.radio_value(
                                    &mut self.timing.fade_mode,
                                    FadeMode::Beats4,
                                    "4 Beats",
                                );
                                ui.radio_value(
                                    &mut self.timing.fade_mode,
                                    FadeMode::Beats16,
                                    "16 Beats",
                                );
                            });
                            ui.label(RichText::new("Fade Mode").heading());
                        });
                    }
                    None => {
                        ui.label("There's no Scene to configure.");
                    }
                }

                match action {
                    Action::None => (),
                    Action::Delete => {
                        let kind = self
                            .pipeline
                            .remove_scene(self.selected_scene)
                            .map(|scene| scene.kind);

                        self.selected_scene = self
                            .pipeline
                            .scenes()
                            .into_iter()
                            .find(|(_index, scene)| match kind {
                                Some(kind) => scene.kind == kind,
                                None => true,
                            })
                            .map(|(index, _scene)| index)
                            .unwrap_or_default();
                    }
                    Action::Clone => {
                        if let Some(index) =
                            self.pipeline
                                .scene(self.selected_scene)
                                .cloned()
                                .map(|mut scene| {
                                    scene.active = false;
                                    self.pipeline.add_scene(scene)
                                })
                        {
                            self.selected_scene = index;
                        }
                    }
                    Action::Move => {
                        if let Some(scene) = self.pipeline.scene(self.selected_scene) {
                            scene.kind.switch()
                        };
                    }
                    Action::InitGpu => {
                        self.pipeline.init_gpu();
                    }
                }
            });
    }
}

enum Action {
    None,
    Delete,
    Clone,
    Move,
    InitGpu,
}
