use crate::{animation::ColorPalette, scene::Scene};

use super::App;
use egui::{
    Align, Button, Checkbox, Color32, Context, DragValue, Image, Layout, Margin, Rect, RichText,
    Rounding, Sense, Shape, Slider, TextureId, Ui, Vec2, Widget,
};

impl App {
    pub fn scenes(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Scenes").heading());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut self.show_scenes_svg, RichText::new("SVG").heading());
                    ui.add(
                        Slider::new(&mut self.scene_size, 100.0..=512.0)
                            .show_value(false)
                            .text(RichText::new("Size").heading()),
                    );
                });
            });

            let svg = self
                .svg
                .as_ref()
                .filter(|_| self.show_scenes_svg)
                .map(|svg| svg.image().texture_id(ctx));

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        crate::get_pipeline!(pipeline);
                        for (index, scene) in pipeline.scenes() {
                            ui.add_sized(
                                Vec2::new(self.scene_size + 40.0, self.scene_size + 60.0),
                                SceneWidget {
                                    selected_scene: &mut self.selected_scene,
                                    index,
                                    scene,
                                    svg,
                                    scene_size: self.scene_size,
                                },
                            );
                        }
                    });
                });
        });
    }
}

struct SceneWidget<'a> {
    selected_scene: &'a mut usize,
    index: usize,
    scene: &'a mut Scene,
    svg: Option<TextureId>,
    scene_size: f32,
}

impl<'a> Widget for SceneWidget<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let mut slider_rect = None;
        let mut checkbox_rect = None;
        let mut beat_progression_offset_rect = None;

        let response = egui::Frame::none()
            .fill(if *self.selected_scene == self.index {
                Color32::GREEN
            } else {
                Color32::TRANSPARENT
            })
            .inner_margin(Margin::from(10.0))
            .rounding(Rounding::from(4.0))
            .show(ui, |ui| {
                egui::Frame::none()
                    .fill(if self.scene.artnet_extraction {
                        Color32::RED
                    } else {
                        Color32::DARK_GRAY
                    })
                    .inner_margin(Margin::from(10.0))
                    .show(ui, |ui| {
                        let size = Vec2::splat(self.scene_size);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.set_max_width(size.x + 28.0);
                                ui.add(
                                    super::config::group::button(self.scene.group(), false)
                                        .sense(Sense::hover()),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    checkbox_rect = Some(ui.checkbox(&mut false, "").rect);
                                    beat_progression_offset_rect = Some(
                                        ui.add(
                                            DragValue::new(&mut self.scene.beat_progression_offset)
                                                .speed(0.01)
                                                .clamp_range(0.0..=1.0)
                                                .custom_formatter(|n, _| {
                                                    format!("{:.0} %", n * 100.0)
                                                }),
                                        )
                                        .rect,
                                    );
                                    color_band(ui, &mut self.scene.palette);
                                });
                            });
                            ui.horizontal(|ui| {
                                let res = ui.image(self.scene.texture_id(), size);
                                if let Some(svg_texture_id) = self.svg {
                                    ui.put(res.rect, Image::new(svg_texture_id, size));
                                }
                                slider_rect = Some(
                                    ui.allocate_rect(
                                        Rect::from_min_max(
                                            {
                                                let mut pos = ui.next_widget_position();
                                                pos.y -= size.y / 2.0;
                                                pos
                                            },
                                            {
                                                let mut pos = ui.next_widget_position();
                                                pos.x += 16.0;
                                                pos.y += size.y / 2.0;
                                                pos
                                            },
                                        ),
                                        Sense::hover(),
                                    )
                                    .rect,
                                );
                            });
                        })
                    })
            })
            .response;
        if ui
            .put(response.rect, Button::new("").fill(Color32::TRANSPARENT))
            .clicked()
        {
            *self.selected_scene = self.index;
        }

        if let Some(slider_rect) = slider_rect {
            ui.spacing_mut().slider_width = slider_rect.height();
            ui.put(
                slider_rect,
                Slider::new(&mut self.scene.opacity, 0.0..=1.0)
                    .vertical()
                    .show_value(false),
            )
            .on_hover_text("Scene Dimmer");
        }

        if let Some(checkbox_rect) = checkbox_rect {
            ui.put(
                checkbox_rect,
                Checkbox::new(&mut self.scene.artnet_extraction, ""),
            )
            .on_hover_text("Enable Scene");
        }

        if let Some(beat_progression_offset_rect) = beat_progression_offset_rect {
            ui.put(
                beat_progression_offset_rect,
                DragValue::new(&mut self.scene.beat_progression_offset)
                    .speed(0.01)
                    .clamp_range(0.0..=1.0)
                    .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                    .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0)),
            )
            .on_hover_text("Beat Progression Offset");
        }

        response
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
                    pos.x -= ui.available_width();
                    pos.y -= 9.0;
                    pos
                },
                {
                    let mut pos = ui.next_widget_position();
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
                        pos.x -= 1.0 + width_per_color * (i + 1) as f32;
                        pos.y -= 8.0;
                        pos
                    },
                    {
                        let mut pos = ui.next_widget_position();
                        pos.x -= 1.0 + width_per_color * i as f32;
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
