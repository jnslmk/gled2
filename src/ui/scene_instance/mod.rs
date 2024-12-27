pub mod widget;

use super::{action::UiAction, ChangeButton};
use crate::storage::asset::scene::instance::SceneInstance;
use egui::{Button, Checkbox, Color32, Context, Margin, Modifiers};

impl SceneInstance {
    pub fn config_ui(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        let width = ui.available_width() - 20.0;
        ui.horizontal(|ui| {
            egui::Frame::none()
                .inner_margin(Margin::from(3.0))
                .show(ui, |ui| {
                    ui.set_max_width(width / 2.0);
                    ui.vertical(|ui| {
                        ui.label("Activation Input");
                        self.activation_input.change_button(ui);

                        ui.label("Flash Input");
                        self.flash_input.change_button(ui);

                        ui.label("Dimmer Input");
                        self.dimmer_input.change_button(ui);
                    });
                });
            egui::Frame::none()
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

                        ui.label("Beat offset");
                        ui.vertical_centered_justified(|ui| {
                            self.beat_progression_offset.change_button(ui);
                        });
                    });
                });
        });

        egui::Frame::none()
            .inner_margin(Margin::from(6.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                self.groups.change_button(ui);
            });

        egui::Frame::none()
            .inner_margin(Margin::from(6.0))
            .show(ui, |ui| {
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
                        .add(Button::new("🗑 Remove Scene").fill(Color32::DARK_RED))
                        .clicked()
                        || {
                            !ctx.wants_keyboard_input()
                                && ctx.input_mut(|i| {
                                    i.consume_key(Modifiers::default(), egui::Key::Backspace)
                                        || i.consume_key(Modifiers::default(), egui::Key::Delete)
                                })
                        }
                    {
                        UiAction::DeleteSelectedSceneInstance.enqueue();
                    }
                });
            });
    }
}
