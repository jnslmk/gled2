pub mod widget;

use super::{ChangeButton, action::UiAction};
use crate::{
    storage::asset::{Asset, scene::instance::SceneInstance},
    ui::effect::widget::EffectWidget,
};
use egui::{
    Button, Checkbox, Color32, Frame, Label, Margin, Modifiers, Rect, ScrollArea, TopBottomPanel,
    Vec2,
};
use egui_modal::Modal;

impl SceneInstance {
    pub fn config_ui(&mut self, ui: &mut egui::Ui) {
        TopBottomPanel::bottom("Scene Instance action buttons")
            .resizable(false)
            .frame(Frame::NONE.inner_margin(Margin::from(6.0)))
            .show_inside(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    if ui
                        .add(Button::new("🗐 Duplicate Scene").fill(Color32::DARK_BLUE))
                        .clicked()
                    {
                        UiAction::CloneSelectedSceneInstance.enqueue();
                    }
                });

                ui.vertical_centered_justified(|ui| {
                    if ui
                        .add(Button::new("↕ Move Scene to other Grid").fill(Color32::DARK_BLUE))
                        .clicked()
                    {
                        UiAction::MoveSelectedSceneToOtherGrid.enqueue();
                    }
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

        let width = ui.available_width() - 20.0;
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

        ScrollArea::vertical().show(ui, |ui| {
            let Some(scene) = Asset::get(self.scene) else {
                ui.colored_label(Color32::RED, "Scene not found!");
                return;
            };

            ui.label("Animations / Setting Overwrites");

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
                    let res = ui.add_sized(
                        Vec2::splat(100.0),
                        EffectWidget {
                            show_group: true,
                            selectable: Some((&mut selected, 1)),
                            effect,
                            effect_state,
                        },
                    );
                    if overwritten {
                        ui.put(
                            Rect::from_two_pos(
                                res.rect.left_bottom() + Vec2::new(10.0, -10.0),
                                res.rect.left_bottom() + Vec2::new(30.0, -30.0),
                            ),
                            Label::new("⚙"),
                        );
                    }
                    let modal = Modal::new(ui.ctx(), format!("effect settings {index}"))
                        .with_close_on_outside_click(true);
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
                                },
                            );
                            ui.vertical(|ui| {
                                let mut effect = effect.to_owned();
                                let changed = effect.config_ui(effect_state, ui, false);
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
                }
            });
        });
    }
}
