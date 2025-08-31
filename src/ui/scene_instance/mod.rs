pub mod widget;

use super::{ChangeButton, action::UiAction};
use crate::{
    pipeline::group::Groups,
    storage::asset::{Asset, scene::instance::SceneInstance},
    ui::effect::widget::EffectWidget,
};
use egui::{
    Button, Checkbox, Color32, Frame, Label, Margin, Modifiers, Rect, ScrollArea, TopBottomPanel,
    Vec2,
};
use egui_modal::Modal;
use std::sync::Arc;

impl SceneInstance {
    pub fn config_ui(
        &mut self,
        ui: &mut egui::Ui,
        svg: Option<egui::TextureHandle>,
        uv: Option<Rect>,
        groups: Groups,
    ) {
        let width = ui.available_width() - 20.0;

        TopBottomPanel::bottom("Scene Instance action buttons")
            .resizable(false)
            .frame(Frame::NONE.inner_margin(Margin::from(6.0)))
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    egui::Frame::NONE.show(ui, |ui| {
                        ui.set_max_width(width / 2.0);
                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .add(Button::new("🗐 Duplicate Scene").fill(Color32::DARK_BLUE))
                                .clicked()
                            {
                                UiAction::CloneSelectedSceneInstance.enqueue();
                            }
                        });
                    });
                    egui::Frame::NONE.show(ui, |ui| {
                        ui.set_max_width(width / 2.0);
                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .add(
                                    Button::new("↕ Move Scene to other Grid")
                                        .fill(Color32::DARK_BLUE),
                                )
                                .clicked()
                            {
                                UiAction::MoveSelectedSceneToOtherGrid.enqueue();
                            }
                        });
                    });
                });

                ui.vertical_centered_justified(|ui| {
                    if ui
                        .add(Button::new("🗑 Remove Scene").fill(Color32::DARK_RED))
                        .clicked()
                        || {
                            !ui.ctx().wants_keyboard_input()
                                && ui.ctx().input_mut(|i| {
                                    i.consume_key(Modifiers::default(), egui::Key::Backspace)
                                        || i.consume_key(Modifiers::default(), egui::Key::Delete)
                                })
                        }
                    {
                        UiAction::DeleteSelectedSceneInstance.enqueue();
                    }
                });
            });

        ui.horizontal(|ui| {
            egui::Frame::NONE
                .inner_margin(Margin::from(3.0))
                .show(ui, |ui| {
                    ui.set_max_width(width / 2.0);
                    ui.vertical(|ui| {
                        ui.label("Activation Input");
                        self.activation_input.change_button(ui);

                        ui.label("Flash Input");
                        self.flash_input.change_button(ui);

                        if self.flash_input.is_some() {
                            ui.label("Set Offset On Flash");
                            ui.checkbox(&mut self.set_offset_on_flash, "");
                        }

                        ui.label("Dimmer Input");
                        self.dimmer_input.change_button(ui);
                    });
                });
            egui::Frame::NONE
                .inner_margin(Margin::from(3.0))
                .show(ui, |ui| {
                    ui.set_max_width(width / 2.0);
                    ui.vertical(|ui| {
                        ui.label("Active");
                        ui.add(Checkbox::new(&mut self.active, ""));

                        ui.label("Opacity");
                        ui.vertical_centered_justified(|ui| {
                            self.opacity.change_button(ui);
                        });

                        ui.label("Ignore Main Dimmer");
                        ui.add(Checkbox::new(&mut self.ignore_main_dimmer, ""));

                        ui.label("Beat offset");
                        ui.vertical_centered_justified(|ui| {
                            self.beat_progression_offset.change_button(ui);
                        });
                    });
                });
        });

        egui::Frame::NONE
            .inner_margin(Margin::from(6.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                self.groups_overwrite.change_button(ui);
            });

        egui::Frame::NONE
            .inner_margin(Margin::from(6.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                self.palette_overwrite.change_button(ui);
            });

        ui.separator();

        ui.label("Animations / Click to overwrite (⚙)");

        let Some(scene) = Asset::get(self.scene) else {
            ui.colored_label(Color32::RED, "Scene not found!");
            return;
        };

        if !self.effect_overwrites.is_empty() {
            ui.horizontal(|ui| {
                egui::Frame::NONE
                    .inner_margin(Margin::from(3.0))
                    .show(ui, |ui| {
                        ui.set_max_width(width / 2.0);
                        ui.vertical_centered_justified(|ui| {
                            if ui.button("Remove all overwrites (⚙)").clicked() {
                                self.effect_overwrites.clear();
                                UiAction::InitGPU.enqueue();
                            }
                        });
                    });
                egui::Frame::NONE
                    .inner_margin(Margin::from(3.0))
                    .show(ui, |ui| {
                        ui.set_max_width(width / 2.0);
                        ui.vertical_centered_justified(|ui| {
                            if ui.button("Save overwrites to scene").clicked() {
                                let mut scene = Arc::unwrap_or_clone(scene.clone());
                                scene.data.effects.iter_mut().enumerate().for_each(
                                    |(index, effect)| {
                                        if let Some(overwrite) =
                                            self.effect_overwrites.remove(&index)
                                        {
                                            *effect = overwrite;
                                        }
                                    },
                                );
                                scene.save();
                                self.effect_overwrites.clear();
                                UiAction::InitGPU.enqueue();
                            }
                        });
                    });
            });
        }

        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut set_overwritten_effect = None;
                for ((index, effect, overwritten), effect_state) in scene
                    .data
                    .effects()
                    .iter()
                    .enumerate()
                    .map(|(index, effect)| {
                        self.effect_overwrites
                            .get(&index)
                            .map(|effect| (index, effect, true))
                            .unwrap_or((index, effect, false))
                    })
                    .zip(self.effect_states.iter_mut())
                {
                    let mut selected = 0;
                    let rect = ui
                        .add_sized(
                            Vec2::splat(100.0),
                            EffectWidget {
                                show_group: true,
                                selectable: Some((&mut selected, 1)),
                                effect,
                                effect_state,
                                svg: svg.clone(),
                                uv,
                                groups: Some(&groups),
                                groups_show_index: false,
                            },
                        )
                        .rect;
                    if overwritten {
                        ui.painter().text(
                            rect.left_bottom() + Vec2::new(12.0, -10.0),
                            egui::Align2::LEFT_BOTTOM,
                            "⚙",
                            egui::TextStyle::Body.resolve(ui.style()),
                            Color32::from_white_alpha(100),
                        );
                    }
                    let modal = Modal::new(ui.ctx(), format!("effect settings {index}"))
                        .with_close_on_outside_click(true);
                    let svg = svg.clone();
                    modal.show(|ui| {
                        modal.title(ui, "Overwrite Effect Settings");
                        ui.set_width(500.0);
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                Vec2::splat(100.0),
                                EffectWidget {
                                    show_group: true,
                                    selectable: None,
                                    effect,
                                    effect_state,
                                    svg: svg.clone(),
                                    uv,
                                    groups: Some(&groups),
                                    groups_show_index: true,
                                },
                            );
                            ui.vertical(|ui| {
                                let mut effect = effect.to_owned();
                                let changed =
                                    effect.config_ui(effect_state, ui, false, svg.clone(), uv);
                                if changed {
                                    if Some(&effect) != scene.data.effects.get(index) {
                                        set_overwritten_effect = Some((index, Some(effect)));
                                    } else {
                                        set_overwritten_effect = Some((index, None));
                                    }
                                }
                            });
                        });
                    });
                    if selected == 1 {
                        modal.open();
                    }
                }

                if let Some((index, effect)) = set_overwritten_effect {
                    if let Some(effect) = effect {
                        self.effect_overwrites.insert(index, effect);
                    } else {
                        self.effect_overwrites.remove(&index);
                    }
                    UiAction::InitGPU.enqueue();
                }
            });
        });
    }
}
