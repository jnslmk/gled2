pub mod grid;
pub mod header;

use super::App;
use egui::Ui;

impl App {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn scenes(&mut self, ui: &mut Ui) {
        let svg = self.svg_mut().and_then(|svg| svg.image(ui.ctx()));

        self.effects_header(ui);
        self.effects_grid(ui, svg)
    }
}
