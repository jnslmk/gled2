use crate::ui::{
    viewport_builder::default_viewport_builder, windows::window_decorations::gled_window_frame,
};
use egui::{Color32, Frame, Id, Layout, Vec2, ViewportId};

#[derive(Default)]
pub struct ErrorsWindow {
    pub entries: Vec<String>,
}

impl ErrorsWindow {
    pub fn update(&mut self, ctx: &egui::Context) {
        if self.entries.is_empty() {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("errors window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(200.0, 150.0))
                .with_minimize_button(false)
                .with_maximize_button(true)
                .with_resizable(true),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.entries.clear();
                    }
                });

                gled_window_frame(ctx, "Errors", |ui| {
                    egui::CentralPanel::default()
                        .frame(Frame::default().fill(Color32::DARK_RED))
                        .show_inside(ui, |ui| {
                            ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                                ui.heading("Errors!");
                                ui.spacing();
                                ui.vertical_centered_justified(|ui| {
                                    for error in &self.entries {
                                        ui.label(error);
                                    }
                                });
                            });
                        });
                });
            },
        );
    }
}
