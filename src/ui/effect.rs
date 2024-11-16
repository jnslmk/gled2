use super::ChangeButton;
use crate::{
    animation::{Animation, AnimationConfig},
    effect::{Effect, EffectState},
};
use egui::{Checkbox, RichText, Slider};
use strum::IntoEnumIterator;

impl Effect {
    pub fn config_ui(&mut self, state: &mut EffectState, ui: &mut egui::Ui) -> bool {
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
            state.update(self);
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

        self.animation.ui(ui, state.texture_id());

        ui.separator();

        changed |= ui
            .add(Checkbox::new(
                &mut self.use_secondary_group,
                "Use Secondary Group",
            ))
            .changed();

        changed
    }
}
