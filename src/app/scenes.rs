mod grid;
mod header;
mod widget;

use super::{preview_uv, App};
use crate::scene::SceneKind;
use egui::{Color32, Context, Margin, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Scenes {
    pub size: f32,
    pub show_svg: bool,
    pub always_render: bool,
}

impl Default for Scenes {
    fn default() -> Self {
        Self {
            size: 100.0,
            show_svg: true,
            always_render: false,
        }
    }
}

impl App {
    pub fn scenes(&mut self, ctx: &Context) {
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
                    self.scenes_header(ui, SceneKind::Background);
                    self.scenes_grid(ctx, ui, SceneKind::Background, svg, uv)
                });
        });
    }
}
