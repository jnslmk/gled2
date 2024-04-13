use super::App;
use crate::effect::Effect;
use egui::{Align, Checkbox, Layout, RichText, Slider, Ui};

impl App {
    pub fn effects_header(&mut self, ui: &mut Ui) {
        let effects = &mut self.persistant_state.effects;

        ui.horizontal(|ui| {
            ui.label(RichText::new("Effects").heading());
            if ui.button("Add").clicked() {
                let effect = Effect::default();
                self.selected_effect = self.pipeline.add_effect(effect);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        self.svg.is_some(),
                        Checkbox::new(&mut effects.show_svg, RichText::new("SVG")),
                    )
                    .changed()
                {
                    self.persistant_state.dirty = true;
                };

                if ui
                    .checkbox(&mut effects.always_render, RichText::new("Render all"))
                    .changed()
                {
                    self.persistant_state.dirty = true;
                };
                if ui
                    .add(
                        Slider::new(&mut effects.size, 90.0..=500.0)
                            .show_value(false)
                            .text(RichText::new("Size")),
                    )
                    .changed()
                {
                    self.persistant_state.dirty = true;
                }
            });
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.pipeline.auto_mode_active, "Auto Mode");
            ui.add_enabled(
                self.pipeline.auto_mode_active,
                Slider::new(&mut self.pipeline.auto_mode_seconds, 1..=240)
                    .custom_formatter(|n, _| format!("{} s", n)),
            );
            ui.label("Max Effects:");
            ui.add_enabled(
                self.pipeline.auto_mode_active,
                Slider::new(&mut self.pipeline.auto_mode_max_effects, 1..=10),
            );
        });
    }
}
