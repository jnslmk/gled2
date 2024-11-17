mod widget;

use super::ChangeButton;
use crate::{
    effect::{Effect, EffectState},
    storage::Asset,
};
use egui::{Checkbox, Slider};

pub use widget::EffectWidget;

impl Effect {
    pub fn config_ui(
        &mut self,
        state: &mut EffectState,
        ui: &mut egui::Ui,
        allow_animation_change: bool,
    ) -> bool {
        let mut changed = false;

        ui.label("Animation");
        ui.add_enabled_ui(allow_animation_change, |ui| {
            ui.vertical_centered_justified(|ui| {
                if self.animation.change_button(ui) {
                    state.update(self);
                }
            });
        });

        ui.label("Progression");
        ui.vertical_centered_justified(|ui| {
            changed |= self.beat_progression.change_button(ui);
        });

        ui.label("Colorshift");
        ui.vertical_centered_justified(|ui| {
            changed |= self.color_shift.change_button(ui);
        });

        ui.label("Opacity");
        ui.vertical_centered_justified(|ui| {
            changed |= self.opacity.change_button(ui);
        });

        ui.label("Beat offset");
        ui.vertical_centered_justified(|ui| {
            changed |= self.beat_progression_offset.change_button(ui);
        });

        ui.label("Speed");
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = ui.available_width() - 60.0;
            changed |= ui
                .add(
                    Slider::new(&mut self.speed_exponent, -8..=8)
                        .step_by(1.0)
                        .custom_formatter(|n, _| {
                            format!(
                                "{} x",
                                if n > 0.0 {
                                    2i32.pow(n as u32).to_string()
                                } else if n > -0.1 {
                                    "1".to_string()
                                } else {
                                    format!("1/{}", 2i32.pow((-n) as u32))
                                }
                            )
                        })
                        .custom_parser(|s| {
                            let s = s.split(' ').next().unwrap_or(s);
                            let s = s.strip_suffix('x').unwrap_or(s);
                            let n = if let Some(n) =
                                s.strip_prefix("1/").and_then(|s| s.parse::<f32>().ok())
                            {
                                1f32 / n
                            } else {
                                s.parse::<f32>().ok()?
                            };

                            Some(n.log2() as f64)
                        }),
                )
                .changed();
        });

        if let Some(animation) = self.animation.and_then(Asset::get) {
            let rendered = state.texture_id();
            changed |= animation
                .data
                .config_ui(&mut state.animation_config, ui, rendered);
        }

        ui.separator();

        ui.label("Use secondary group");
        changed |= ui
            .add(Checkbox::new(&mut self.use_secondary_group, ""))
            .changed();

        changed
    }
}
