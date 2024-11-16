use super::ChangeButton;
use crate::{
    animation::{Animation, AnimationConfig},
    effect::{Effect, EffectState},
};
use egui::Checkbox;
use strum::IntoEnumIterator;

impl Effect {
    pub fn config_ui(&mut self, state: &mut EffectState, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Animation");
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

        ui.horizontal(|ui| {
            changed |= self.beat_progression.change_button(ui);
            ui.label("Progression")
        });

        ui.horizontal(|ui| {
            changed |= self.color_shift.change_button(ui);
            ui.label("Colorshift")
        });

        ui.horizontal(|ui| {
            changed |= self.opacity.change_button(ui);
            ui.label("Opacity")
        });

        ui.horizontal(|ui| {
            changed |= self.beat_progression_offset.change_button(ui);
            ui.label("Beat offset")
        });

        changed |= self.animation.ui(ui, state.texture_id());

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
