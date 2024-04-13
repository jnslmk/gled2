mod color;
pub mod group;

use super::{timing::FadeMode, App};
use crate::{
    animation::{Animation, AnimationConfig, Color},
    assets::find_palette,
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
                            .effects()
                            .into_iter()
                            .flat_map(|(_index, effect)| {
                                find_palette(&effect.palette).colors.clone().into_iter()
                            }),
                    )
                    .collect();

                let svg = self
                    .svg
                    .as_mut()
                    .filter(|_| self.persistant_state.show_preview_svg)
                    .and_then(|svg| svg.image(&mut self.pipeline))
                    .map(|svg| svg.texture_id(ctx));
                let mut action = Action::None;
                match self.pipeline.effect(self.selected_effect) {
                    Some(effect) => {
                        ui.label(RichText::new("Hotkey").heading());
                        ui.vertical_centered_justified(|ui| {
                            ui.menu_button(
                                match effect.hotkey {
                                    Some(hotkey) => format!("{hotkey}"),
                                    None => "Assign".to_string(),
                                },
                                |ui| {
                                    ui.label("Please press a key!");
                                    if let Some(hotkey) = Hotkey::get(ctx, &self.gamepad) {
                                        effect.hotkey = Some(hotkey);
                                        ui.close_menu();
                                    }
                                },
                            );
                        });

                        ui.separator();

                        ui.label(RichText::new("Flash Hotkey").heading());
                        ui.vertical_centered_justified(|ui| {
                            ui.menu_button(
                                match effect.flash_hotkey {
                                    Some(hotkey) => format!("{hotkey}"),
                                    None => "Assign".to_string(),
                                },
                                |ui| {
                                    ui.label("Please press a key!");
                                    if let Some(hotkey) = Hotkey::get(ctx, &self.gamepad) {
                                        effect.flash_hotkey = Some(hotkey);
                                        ui.close_menu();
                                    }
                                },
                            );
                        });
                        ui.separator();

                        ui.label(RichText::new("Colors").heading());
                        color::color_selection(
                            ui,
                            &effect.palette,
                            all_colors,
                            effect.animation.uses_multiple_colors(),
                        );

                        ui.separator();

                        ui.label(RichText::new("Animation").heading());
                        let animation_changed = egui::ComboBox::from_label("Animation")
                            .selected_text(format!("{}", effect.animation))
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                let mut changed = false;
                                for animation in Animation::iter() {
                                    let text = format!("{animation}");
                                    if ui
                                        .selectable_value(&mut effect.animation, animation, text)
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
                            effect.reset_gpu_state();
                            action = Action::InitGpu;
                        }

                        ui.add(Checkbox::new(&mut effect.active, "Active"));
                        ui.add(
                            Slider::new(&mut effect.opacity, 0.0..=1.0)
                                .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0))
                                .text("Opacity"),
                        );
                        ui.add(
                            Slider::new(&mut effect.beat_progression_offset, 0.0..=1.0)
                                .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0))
                                .text("Beat offset"),
                        );

                        effect.config_ui(ui, svg);

                        ui.separator();

                        ui.label(RichText::new("Group").heading());
                        group::selection(ui, &mut effect.group);

                        ui.separator();

                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .add(Button::new("🗐 Duplicate Effect").fill(Color32::DARK_BLUE))
                                .clicked()
                            {
                                action = Action::Clone;
                            }
                        });
                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .add(
                                    Button::new("🗑 Remove Effect")
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
                        ui.label("There's no Effect to configure.");
                    }
                }

                match action {
                    Action::None => (),
                    Action::Delete => {
                        self.pipeline.remove_effect(self.selected_effect);

                        self.selected_effect = self
                            .pipeline
                            .effects()
                            .first()
                            .map(|(index, _effect)| *index)
                            .unwrap_or_default();
                    }
                    Action::Clone => {
                        if let Some(index) = self
                            .pipeline
                            .effect(self.selected_effect)
                            .cloned()
                            .map(|mut effect| {
                                effect.active = false;
                                self.pipeline.add_effect(effect)
                            })
                        {
                            self.selected_effect = index;
                        }
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
    InitGpu,
}
