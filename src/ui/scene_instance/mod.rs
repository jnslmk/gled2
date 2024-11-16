pub mod widget;

use super::{action::Action, ChangeButton};
use crate::scene_instance::SceneInstance;
use egui::{Button, Checkbox, Color32, Context, Margin, Modifiers, Stroke};

impl SceneInstance {
    pub fn config_ui(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(Margin::from(6.0))
            .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
            .show(ui, |ui| {
                ui.label("Selection Input");
                self.selection_input.change_button(ui);

                ui.label("Flash Input");
                self.flash_input.change_button(ui);

                ui.label("Dimmer Input");
                self.dimmer_input.change_button(ui);
            });

        ui.add_space(10.0);

        egui::Frame::none()
            .inner_margin(Margin::from(6.0))
            .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
            .show(ui, |ui| {
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

        ui.add_space(10.0);

        egui::Frame::none()
            .inner_margin(Margin::from(6.0))
            .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    if ui
                        .add(Button::new("🗐 Duplicate Scene").fill(Color32::DARK_BLUE))
                        .clicked()
                    {
                        Action::CloneSelectedSceneInstance.enqueue();
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
                        Action::DeleteSelectedSceneInstance.enqueue();
                    }
                });
            });
    }
}
