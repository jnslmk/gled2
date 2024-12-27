pub mod grid;
pub mod header;

use super::{svg::Svg, App};
use crate::storage::asset::project::scene_instance_path::SceneInstancePath;
use egui::{Margin, Ui};

impl App {
    pub fn scenes(&mut self, ui: &mut Ui, path: SceneInstancePath) {
        let svg = self
            .svg_mut()
            .and_then(|svg| svg.image())
            .map(|image| image.texture_id(ui.ctx()));
        let uv = Svg::preview_uv();
        egui::Frame::none()
            .inner_margin(Margin::from(6.0))
            .show(ui, |ui| {
                self.effects_header(ui, path);
                self.effects_grid(ui, svg, uv, path)
            });
    }
}
