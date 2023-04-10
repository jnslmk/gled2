use super::App;
use egui::{Align, Color32, Context, Image, Layout, RichText};

impl App {
    pub fn preview(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("preview")
            .default_height(300.0)
            .resizable(self.show_preview)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_preview, RichText::new("Preview").heading());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.checkbox(&mut self.show_preview_svg, RichText::new("SVG").heading());
                    });
                });

                if self.show_preview {
                    ui.set_min_height(300.0);

                    crate::get_pipeline!(pipeline);
                    let size = egui::Vec2::splat(ui.available_height());

                    let res = self
                        .svg
                        .as_ref()
                        .filter(|_| self.show_preview_svg)
                        .map(|svg| {
                            ui.add(
                                Image::new(svg.image().texture_id(ctx), size)
                                    .bg_fill(Color32::BLACK),
                            )
                        });
                    let mut preview = Image::new(pipeline.preview_texture_id(), size);
                    if !self.show_preview_svg {
                        preview = preview.bg_fill(Color32::BLACK);
                    }
                    match res {
                        Some(res) => {
                            ui.put(res.rect, preview);
                        }
                        None => {
                            ui.add(preview);
                        }
                    }
                } else {
                    ui.set_min_height(10.0);
                }
            });
    }
}
