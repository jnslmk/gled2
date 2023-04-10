use super::App;
use egui::Context;

impl App {
    pub fn deck_selection(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("deck_selection")
            .default_height(100.0)
            .show(ctx, |ui| {
                ui.label("TODO: Deck selection");
            });
    }
}
