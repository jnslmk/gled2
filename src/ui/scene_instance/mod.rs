pub mod widget;

use super::{action::Action, ChangeButton};
use crate::scene_instance::SceneInstance;
use egui::{Button, Checkbox, Color32, Context, Modifiers, RichText, Slider};

impl SceneInstance {
    pub fn config_ui(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        ui.label(RichText::new("Selection Input").heading());
        self.selection_input.change_button(ui);

        ui.separator();

        ui.label(RichText::new("Flash Input").heading());
        self.flash_input.change_button(ui);

        ui.separator();

        ui.label(RichText::new("Dimmer Input").heading());
        self.dimmer_input.change_button(ui);

        ui.separator();

        ui.add(Checkbox::new(&mut self.active, "Active"));
        ui.add(
            Slider::new(&mut self.opacity, 0.0..=1.0)
                .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0))
                .text("Opacity"),
        );
        ui.add(
            Slider::new(&mut self.beat_progression_offset, 0.0..=1.0)
                .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0))
                .text("Beat offset"),
        );

        ui.separator();

        ui.vertical_centered_justified(|ui| {
            if ui
                .add(Button::new("🗐 Duplicate Scene Instance").fill(Color32::DARK_BLUE))
                .clicked()
            {
                Action::CloneSelectedSceneInstance.enqueue();
            }
        });
        ui.vertical_centered_justified(|ui| {
            if ui
                .add(
                    Button::new("🗑 Remove Scene Instance")
                        .fill(Color32::DARK_RED)
                        .shortcut_text("Del"),
                )
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
    }
}
