use super::App;
use egui::{Align, Color32, Context, Image, Layout, RichText, Slider};

impl App {
    pub fn preview(&mut self, ctx: &Context) {
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
                    if !self.show_preview {
                        ui.add_space(ui.available_width() / 2.0 - 200.0);
                        ui.spacing_mut().slider_width = 300.0;
                        ui.add(
                            Slider::new(&mut self.main_dimmer, 0.0..=1.0)
                                .show_value(false)
                                .text("main"),
                        );
                    }
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

                        ui.add_space(ui.available_width() - 36.0);
                        ui.separator();
                        ui.spacing_mut().slider_width = ui.available_height() - 12.0;
                        ui.add(
                            Slider::new(&mut self.main_dimmer, 0.0..=1.0)
                                .vertical()
                                .text("main")
                                .show_value(false),
                        );
                    });
                }
            });
    }
}
