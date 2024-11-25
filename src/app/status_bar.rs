use super::App;
use crate::temperature::temperature;
use egui::{Context, Label, Layout};

impl App {
    pub fn status_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("Stauts_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(framerate) = self.timing.framerate() {
                    ui.add(Label::new(format!("{framerate:.0} fps")));
                }
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(Label::new(temperature()))
                });
            });
        });
    }
}
