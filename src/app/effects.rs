pub mod grid;
pub mod header;

use super::{App, svg::Svg};
use crate::storage::asset::project::scene_instance_path::SceneInstancePath;
use egui::{Rect, Ui, Vec2};
use egui_tiles::UiResponse;

impl App {
    pub fn scenes(&mut self, ui: &mut Ui, path: SceneInstancePath) -> UiResponse {
        let drag_button_rect = {
            let mut pos = ui.next_widget_position();
            pos.y += 2.0;
            pos.x += ui.available_width() - 22.0;
            Rect::from_min_size(pos, Vec2::splat(20.0))
        };

        let svg = self
            .svg_mut()
            .and_then(|svg| svg.image())
            .map(|image| image.texture_id(ui.ctx()));
        let uv = Svg::preview_uv();

        self.effects_header(ui, path);
        self.effects_grid(ui, svg, uv, path);

        if ui
            .put(
                drag_button_rect,
                egui::Button::new("✊").sense(egui::Sense::drag()),
            )
            .drag_started()
        {
            UiResponse::DragStarted
        } else {
            UiResponse::None
        }
    }
}
