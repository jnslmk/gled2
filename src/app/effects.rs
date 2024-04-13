mod grid;
mod header;
mod widget;

use super::{preview_uv, App};
use egui::{Color32, Context, Margin, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Effects {
    pub size: f32,
    pub show_svg: bool,
    pub always_render: bool,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            size: 100.0,
            show_svg: true,
            always_render: false,
        }
    }
}

impl App {
    pub fn effects(&mut self, ctx: &Context) {
        let svg = self
            .svg
            .as_mut()
            .and_then(|svg| svg.image(&mut self.pipeline))
            .map(|image| image.texture_id(ctx));
        let uv = preview_uv();
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::none()
                .inner_margin(Margin::from(6.0))
                .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
                .show(ui, |ui| {
                    self.effects_header(ui);
                    self.effects_grid(ctx, ui, svg, uv)
                });
        });
    }
}
