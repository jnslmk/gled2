mod color;

use super::{timing::FadeMode, App};
use egui::{Context, Layout, RichText};

impl App {
    pub fn config(&mut self, ctx: &Context) {
        egui::SidePanel::left("left")
            .resizable(true)
            .default_width(280.0)
            .min_width(280.0)
            .max_width(ctx.used_rect().width() - 950.0)
            .show(ctx, |ui| {
                let svg = self
                    .svg
                    .as_mut()
                    .filter(|_| self.persistant_state.show_preview_svg)
                    .and_then(|svg| svg.image(&mut self.pipeline))
                    .map(|svg| svg.texture_id(ctx));

                match self.pipeline.effect(self.selected_effect) {
                    Some(effect) => {
                        effect.config_ui(ctx, ui, svg);
                    }
                    None => {
                        ui.label("There's no Effect to configure.");
                    }
                }

                ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
                    if let Some(framerate) = self.timing.framerate() {
                        ui.label(format!("{framerate:.01} fps"));
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.timing.fade_mode, FadeMode::Instant, "Instant");
                        ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beat, "1 Beat");
                        ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beats4, "4 Beats");
                        ui.radio_value(&mut self.timing.fade_mode, FadeMode::Beats16, "16 Beats");
                    });
                    ui.label(RichText::new("Fade Mode").heading());
                });
            });
    }
}
