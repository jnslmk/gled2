use crate::{animation::ColorPalette, scene::Scene};

use super::App;
use egui::{
    Align, Button, Color32, Context, Image, Layout, Margin, Rect, RichText, Rounding, Sense, Shape,
    TextureId, Ui,
};

impl App {
    pub fn scenes(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Scenes").heading());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut self.show_scenes_svg, RichText::new("SVG").heading());
                });
            });

            let svg = self
                .svg
                .as_ref()
                .filter(|_| self.show_scenes_svg)
                .map(|svg| svg.image().texture_id(ctx));

            //egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                crate::get_pipeline!(pipeline);
                for (index, scene) in pipeline.scenes() {
                    self.selectable_scene(ui, index, scene, svg);
                }
            });
            //});
        });
    }

    fn selectable_scene(
        &mut self,
        ui: &mut Ui,
        index: usize,
        scene: &mut Scene,
        svg: Option<TextureId>,
    ) {
        let rect = egui::Frame::none()
            .fill(if self.selected_scene == index {
                Color32::GREEN
            } else {
                Color32::TRANSPARENT
            })
            .inner_margin(Margin::from(10.0))
            .rounding(Rounding::from(4.0))
            .show(ui, |ui| {
                self.scene(ui, scene, svg);
            })
            .response
            .rect;
        if ui
            .put(rect, Button::new("").fill(Color32::TRANSPARENT))
            .clicked()
        {
            self.selected_scene = index;
        }
    }

    fn scene(&mut self, ui: &mut Ui, scene: &mut Scene, svg: Option<TextureId>) {
        let size = egui::Vec2::splat(256.0);

        egui::Frame::none()
            .fill(egui::Color32::RED)
            .inner_margin(Margin::from(10.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.set_max_width(size.x);
                        ui.add(
                            super::config::group::button(scene.group(), false)
                                .sense(Sense::hover()),
                        );
                        color_band(ui, &mut scene.palette);
                    });
                    let res = ui.image(scene.texture_id(), size);
                    if let Some(svg_texture_id) = svg {
                        ui.put(res.rect, Image::new(svg_texture_id, size));
                    }
                });
            });
    }
}

fn color_band(ui: &mut Ui, palette: &mut ColorPalette) {
    let colors = palette.colors();
    let width_per_color = (ui.available_width() - 2.0) / colors.len() as f32;
    ui.painter().add(Shape::Vec(
        std::iter::once(Shape::rect_filled(
            Rect::from_min_max(
                {
                    let mut pos = ui.next_widget_position();
                    pos.y -= 9.0;
                    pos
                },
                {
                    let mut pos = ui.next_widget_position();
                    pos.x += ui.available_width();
                    pos.y += 9.0;
                    pos
                },
            ),
            Rounding::none(),
            Color32::BLACK,
        ))
        .chain(colors.iter().enumerate().map(|(i, color)| {
            Shape::rect_filled(
                Rect::from_min_max(
                    {
                        let mut pos = ui.next_widget_position();
                        pos.x += 1.0 + width_per_color * i as f32;
                        pos.y -= 8.0;
                        pos
                    },
                    {
                        let mut pos = ui.next_widget_position();
                        pos.x += 1.0 + width_per_color * (i + 1) as f32;
                        pos.y += 8.0;
                        pos
                    },
                ),
                Rounding::none(),
                color,
            )
        }))
        .collect(),
    ));
}
