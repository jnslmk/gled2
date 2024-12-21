use super::{App, Svg};
use crate::preview::Preview;
use egui::{load::SizedTexture, Color32, Context, Image, Vec2};

impl App {
    pub fn preview(&mut self, ctx: &Context) {
        if let Some(uv) = Svg::preview_uv() {
            egui::CentralPanel::default().show(ctx, |ui| {
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

                let res = self.svg_mut().and_then(|svg| svg.image()).map(|image| {
                    ui.add(
                        Image::new(SizedTexture::new(image.texture_id(ctx), size))
                            .uv(uv)
                            .bg_fill(Color32::BLACK),
                    )
                });
                let preview = Image::new(SizedTexture::new(Preview::texture_id(), size)).uv(uv);
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
    }
}
