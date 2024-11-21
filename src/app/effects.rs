mod grid;
mod header;

use super::{preview_uv, App};
use egui::{Color32, Context, Margin, Stroke};

impl App {
    pub fn effects(&mut self, ctx: &Context) {
        let svg = self
            .project
            .svg
            .as_mut()
            .and_then(|svg| svg.image())
            .map(|image| image.texture_id(ctx));
        let uv = preview_uv();
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::none()
                .inner_margin(Margin::from(6.0))
                .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
                .show(ui, |ui| {
                    self.effects_header(ui);
                    self.effects_grid(ui, svg, uv)
                });
        });
    }
}
