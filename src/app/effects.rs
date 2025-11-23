pub mod grid;
pub mod header;

use super::App;
use crate::storage::asset::project::DeckPath;
use egui::Ui;

impl App {
    pub fn scenes(&mut self, ui: &mut Ui, deck_path: DeckPath) {
        let svg = self.svg_mut().and_then(|svg| svg.image(ui.ctx()));

        self.effects_header(ui, deck_path);
        match deck_path {
            DeckPath::Grid => self.effects_grid(ui, svg),
            DeckPath::Quick => self.effects_quick(ui, svg),
        }
    }
}
