use egui::{Align, Context, Layout};

use super::App;

impl App {
    pub fn no_project(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.heading("No project loaded!");
                ui.add_space(10.0);
                if ui.button("Load Project").clicked() {
                    self.windows.projects.open();
                }
            });
        });
    }
}
