pub mod grid;
pub mod header;

use super::{App, svg::Svg};
use crate::storage::asset::project::{DeckPath, scene_instance_path::SceneInstancePath};
use egui::Ui;

impl App {
    pub fn scenes(&mut self, ui: &mut Ui, path: SceneInstancePath) {
        let svg = self.svg_mut().and_then(|svg| svg.image(ui.ctx()));
        let uv = Svg::preview_uv();

        self.effects_header(ui, path);
        match path.deck_path {
            DeckPath::Grid => self.effects_grid(ui, svg, uv),
            DeckPath::Quick => self.effects_quick(ui, svg, uv),
        }
    }
}
