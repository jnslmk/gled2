use super::{action::Action, group::group_selection, ChangeButton};
use crate::{
    animation::{Animation, AnimationConfig},
    effect::Effect,
};
use egui::{Button, Checkbox, Color32, Context, Modifiers, RichText, Slider, TextureId};
use strum::IntoEnumIterator;

impl Effect {
    pub fn config_ui(&mut self, ctx: &Context, ui: &mut egui::Ui, svg: Option<TextureId>) {
        ui.label(RichText::new("Selection Input").heading());
        self.selection_input.change_button(ui);

        ui.separator();

        ui.label(RichText::new("Flash Input").heading());
        self.flash_input.change_button(ui);

        ui.separator();

        ui.label(RichText::new("Dimmer Input").heading());
        self.dimmer_input.change_button(ui);

        ui.separator();

        ui.label(RichText::new("Color Shift").heading());
        ui.vertical_centered_justified(|ui| {
            self.color_shift.change_button(ui);
        });

        ui.separator();

        ui.label(RichText::new("Animation").heading());
        let animation_changed = egui::ComboBox::from_label("Animation")
            .selected_text(format!("{}", self.animation))
            .width(150.0)
            .show_ui(ui, |ui| {
                let mut changed = false;
                for animation in Animation::iter() {
                    let text = format!("{animation}");
                    if ui
                        .selectable_value(&mut self.animation, animation, text)
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
            self.reset_gpu_state();
            Action::InitGpu.enqueue();
        }

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

        self.animation.ui(ui, self.texture_id(), svg);

        ui.separator();

        ui.label(RichText::new("Group").heading());
        group_selection(ui, &mut self.group);

        ui.separator();

        ui.vertical_centered_justified(|ui| {
            if ui
                .add(Button::new("🗐 Duplicate Effect").fill(Color32::DARK_BLUE))
                .clicked()
            {
                Action::CloneSelectedEffect.enqueue();
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
                            i.consume_key(Modifiers::default(), egui::Key::Backspace)
                                || i.consume_key(Modifiers::default(), egui::Key::Delete)
                        })
                }
            {
                Action::DeleteSelectedEffect.enqueue();
            }
        });
    }
}
