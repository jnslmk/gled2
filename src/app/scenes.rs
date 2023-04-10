use super::App;
use egui::{Align, Context, Image, Layout, RichText};

impl App {
    pub fn scenes(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Scenes").heading());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut self.show_scenes_svg, RichText::new("SVG").heading());
                });
            });

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let size = egui::Vec2::splat(256.0);
                    crate::get_pipeline!(pipeline);
                    for (_index, scene) in pipeline.scenes() {
                        let res = ui.image(scene.texture_id(), size);
                        if let Some(svg) = self.svg.as_ref().filter(|_| self.show_scenes_svg) {
                            ui.put(res.rect, Image::new(svg.image().texture_id(ctx), size));
                        }
                    }
                });
            });
        });
    }
}
