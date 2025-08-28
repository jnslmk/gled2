use super::{App, svg::Svg};
use crate::pipeline::preview::Preview;
use egui::{Color32, Image, Vec2, load::SizedTexture};

impl App {
    pub fn preview(&mut self, ui: &mut egui::Ui) {
        if let Some(uv) = Svg::preview_uv() {
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

            let res =
                self.svg_mut()
                    .and_then(|svg| svg.image(ui.ctx()))
                    .map(|svg_texture_handle| {
                        ui.add(
                            Image::new(SizedTexture::new(svg_texture_handle.id(), size))
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
        }
    }
}
