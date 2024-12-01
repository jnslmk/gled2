use super::App;
use crate::{storage::polynomials_fitting, temperature::temperature};
use egui::{Context, Label, Layout, Spinner, ViewportId};

impl App {
    pub fn status_bar(&mut self, ctx: &Context, viewport_id: Option<ViewportId>) {
        egui::TopBottomPanel::bottom(format!("{viewport_id:?} status bar")).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(framerate) = self.timing.framerate() {
                    ui.add(Label::new(format!("{framerate:.0} fps")));
                }

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(Label::new(temperature()));
                    match polynomials_fitting() {
                        0 => (),
                        n => {
                            ui.add_space(4.0);
                            ui.add(Spinner::new()).on_hover_ui(|ui| {
                                ui.label(format!("Fitting {n} polynomials"));
                            });
                            ui.add_space(4.0);
                        }
                    }
                });
            });
        });
    }
}
