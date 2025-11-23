use super::App;
use crate::pipeline::preview::Preview;
use egui::{Color32, Image, Vec2, load::SizedTexture};

impl App {
    pub fn preview(&mut self, ui: &mut egui::Ui) {
        let size = Vec2::splat(ui.available_height().min(ui.available_width()));

        let res = self
            .svg_mut()
            .and_then(|svg| svg.image(ui.ctx()))
            .map(|svg_texture_handle| {
                ui.add(
                    Image::new(SizedTexture::new(svg_texture_handle.id(), size))
                        .bg_fill(Color32::BLACK),
                )
            });
        let preview = Image::new(SizedTexture::new(Preview::texture_id(), size));
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
