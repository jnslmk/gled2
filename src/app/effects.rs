pub mod grid;

use super::App;
use egui::Ui;

impl App {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn scenes(&mut self, ui: &mut Ui) {
        let svg_texture = self.svg_texture(ui.ctx());

        self.effects_grid(ui, svg_texture)
    }
}
