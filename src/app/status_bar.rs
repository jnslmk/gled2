use super::App;
use crate::{storage::polynomials_fitting, temperature::temperature};
use egui::{Context, Label, Layout, Spinner};

impl App {
    pub fn status_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("status bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(framerate) = self.timing.framerate() {
                    ui.add(Label::new(format!("{framerate:.0} fps")));
                }

                match polynomials_fitting() {
                    0 => (),
                    n => {
                        ui.add_space(10.0);
                        ui.add(Label::new(format!("Fitting polynomials: {n}")));
                        ui.add(Spinner::new());
                        ui.add_space(10.0);
                    }
                }

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(Label::new(temperature()))
                });
            });
        });
    }
}
