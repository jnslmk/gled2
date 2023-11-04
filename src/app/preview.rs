use super::App;
use crate::app::preview_uv;
use egui::{load::SizedTexture, Align, Color32, Context, Image, Layout, RichText, Vec2};

impl App {
    pub fn preview(&mut self, ctx: &Context) {
        if let Some(uv) = preview_uv() {
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
                                Image::new(SizedTexture::new(image.texture_id(ctx), size))
                                    .uv(uv)
                                    .bg_fill(Color32::BLACK),
                            )
                        });
                    let mut preview =
                        Image::new(SizedTexture::new(self.pipeline.preview_texture_id(), size))
                            .uv(uv);
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
