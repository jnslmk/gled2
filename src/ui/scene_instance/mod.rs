pub mod widget;

use super::{ChangeButton, action::UiAction};
use crate::storage::asset::scene::instance::SceneInstance;
use egui::{Button, Checkbox, Color32, Margin, Modifiers, Rect, Vec2};
use egui_tiles::UiResponse;

impl SceneInstance {
    pub fn config_ui(&mut self, ui: &mut egui::Ui) -> UiResponse {
        let drag_button_rect = {
            let mut pos = ui.next_widget_position();
            pos.y += 2.0;
            pos.x += ui.available_width() - 22.0;
            Rect::from_min_size(pos, Vec2::splat(20.0))
        };

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
                self.groups.change_button(ui);
            });

        egui::Frame::NONE
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

        if ui
            .put(
                drag_button_rect,
                egui::Button::new("✊").sense(egui::Sense::drag()),
            )
            .drag_started()
        {
            UiResponse::DragStarted
        } else {
            UiResponse::None
        }
    }
}
