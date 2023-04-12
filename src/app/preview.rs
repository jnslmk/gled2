use super::App;
use crate::app::preview_positions;
use egui::{Align, Color32, Context, Image, Layout, RichText, Slider, Vec2};

const BORDER: f32 = 0.03;

impl App {
    pub fn preview(&mut self, ctx: &Context) {
        if let Some(mut uv) = preview_positions().uv() {
            uv.min.y = (uv.min.y - BORDER).max(0.0);
            uv.min.x = (uv.min.x - BORDER).max(0.0);
            uv.max.x = (uv.max.x + BORDER).min(1.0);
            uv.max.y = (uv.max.y + BORDER).min(1.0);

            egui::TopBottomPanel::top("preview")
                .default_height(300.0)
                .exact_height(if self.show_preview {
                    self.preview_size
                } else {
                    10.0
                })
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_preview, RichText::new("Preview").heading());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.checkbox(&mut self.show_preview_svg, RichText::new("SVG").heading());
                            if self.show_preview {
                                ui.add(
                                    Slider::new(&mut self.preview_size, 100.0..=512.0)
                                        .show_value(false)
                                        .text(RichText::new("Size").heading()),
                                );
                            }
                        });
                    });

                    if self.show_preview {
                        ui.horizontal(|ui| {
                            ui.set_min_height(self.preview_size);

                            crate::get_pipeline!(pipeline);

                            let size = if uv.max.x > uv.max.y {
                                Vec2::new(
                                    ui.available_height() * uv.max.x / uv.max.y,
                                    ui.available_height(),
                                )
                            } else {
                                Vec2::new(
                                    ui.available_height() * uv.max.y / uv.max.x,
                                    ui.available_height(),
                                )
                            };

                            let res =
                                self.svg
                                    .as_ref()
                                    .filter(|_| self.show_preview_svg)
                                    .map(|svg| {
                                        ui.add(
                                            Image::new(svg.image().texture_id(ctx), size)
                                                .uv(uv)
                                                .bg_fill(Color32::BLACK),
                                        )
                                    });
                            let mut preview =
                                Image::new(pipeline.preview_texture_id(), size).uv(uv);
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
                        });
                    }
                });
        }
    }
}
