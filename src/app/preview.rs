use super::App;
use crate::app::preview_positions;
use egui::{Align, Color32, Context, Image, Layout, RichText, Vec2};

const BORDER: f32 = 0.03;

impl App {
    pub fn preview(&mut self, ctx: &Context) {
        if let Some(mut uv) = preview_positions().uv() {
            uv.min.y = (uv.min.y - BORDER).max(0.0);
            uv.min.x = (uv.min.x - BORDER).max(0.0);
            uv.max.x = (uv.max.x + BORDER).min(1.0);
            uv.max.y = (uv.max.y + BORDER).min(1.0);

            let preview_rect = egui::TopBottomPanel::top("preview")
                .default_height(self.persistant_state.preview_height)
                .min_height(50.0)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Preview").heading());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .checkbox(
                                    &mut self.persistant_state.show_preview_svg,
                                    RichText::new("SVG").heading(),
                                )
                                .changed()
                            {
                                self.persistant_state.dirty = true;
                            }
                        });
                    });

                    let size = if uv.max.x > uv.max.y {
                        Vec2::new(
                            ui.available_width()
                                .min(ui.available_height() * uv.max.x / uv.max.y),
                            ui.available_height()
                                .min(ui.available_width() * uv.max.y / uv.max.x),
                        )
                    } else {
                        Vec2::new(
                            ui.available_width()
                                .min(ui.available_height() * uv.max.y / uv.max.x),
                            ui.available_height()
                                .min(ui.available_width() * uv.max.x / uv.max.y),
                        )
                    };

                    let res = self
                        .svg
                        .as_mut()
                        .filter(|_| self.persistant_state.show_preview_svg)
                        .and_then(|svg| svg.image(&mut self.pipeline))
                        .map(|image| {
                            ui.add(
                                Image::new(image.texture_id(ctx), size)
                                    .uv(uv)
                                    .bg_fill(Color32::BLACK),
                            )
                        });
                    let mut preview = Image::new(self.pipeline.preview_texture_id(), size).uv(uv);
                    if !self.persistant_state.show_preview_svg {
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
                })
                .response
                .rect;
            if preview_rect.height() != self.persistant_state.preview_height {
                self.persistant_state.preview_height = preview_rect.height();
                self.persistant_state.dirty = true;
            }
        }
    }
}
