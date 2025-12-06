use super::App;
use crate::{pipeline::preview::Preview, ui::brightness_slider::BrightnessSlider};
use egui::{Color32, Frame, Image, Layout, Vec2, load::SizedTexture};

impl App {
    pub fn preview(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();
        ui.horizontal(|ui| {
            let size = Vec2::splat(height.min(ui.available_width()));
            let res =
                self.svg_mut()
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

            if let Some(project) = self.project.as_mut() {
                let mut rect = ui.available_rect_before_wrap();
                *rect.top_mut() += 16.0;
                *rect.left_mut() = rect.right() - 56.0;
                *rect.bottom_mut() -= 16.0;

                ui.put(
                    rect,
                    BrightnessSlider {
                        value: &mut project.main_dimmer,
                        width: 40.0,
                        show_label: true,
                    },
                );
            }
        });
    }
}
