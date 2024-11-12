use super::{action::Action, ChangeButton};
use crate::{animation::Animation, effect::Effect};
use egui::{Button, Checkbox, Color32, RichText, Slider};
use strum::IntoEnumIterator;

impl Effect {
    pub fn config_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.label(RichText::new("Color Shift").heading());
        ui.vertical_centered_justified(|ui| {
            changed |= self.color_shift.change_button(ui);
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
            changed = true;
            //TODO: reset state of preview (we need a preview to change effect settings)
            //self.reset_gpu_state();
            Action::InitGpu.enqueue();
        }

        changed |= ui
            .add(
                Slider::new(&mut self.opacity, 0.0..=1.0)
                    .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                    .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0))
                    .text("Opacity"),
            )
            .changed();
        changed |= ui
            .add(
                Slider::new(&mut self.beat_progression_offset, 0.0..=1.0)
                    .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                    .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0))
                    .text("Beat offset"),
            )
            .changed();

        //TODO: add animation settings back with texture_id of preview (we need a preview for the animation settings)
        //self.animation.ui(ui, self.texture_id());

        ui.separator();

        changed |= ui
            .add(Checkbox::new(
                &mut self.use_secondary_group,
                "Use Secondary Group",
            ))
            .changed();

        ui.separator();

        ui.vertical_centered_justified(|ui| {
            if ui
                .add(Button::new("🗐 Duplicate Effect").fill(Color32::DARK_BLUE))
                .clicked()
            {
                Action::CloneSelectedSceneInstance.enqueue();
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
            {
                Action::DeleteSelectedSceneInstance.enqueue();
            }
        });

        changed
    }
}
