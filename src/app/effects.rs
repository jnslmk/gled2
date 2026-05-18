pub mod grid;

use super::App;
use egui::Ui;

impl App {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn scenes(&mut self, ui: &mut Ui) {
        self.effects_grid(ui)
    }
}
