use super::App;
use crate::{
    animation::ColorPalette,
    scene::{Scene, SceneKind},
};
use egui::{
    Align, Button, Checkbox, Color32, Context, DragValue, Image, Layout, Margin, Rect, RichText,
    Rounding, Sense, Shape, Slider, Stroke, TextureId, Ui, Vec2, Widget,
};
use egui_extras::{Size, StripBuilder};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Scenes {
    pub size: f32,
    pub show_svg: bool,
    pub always_render: bool,
}

impl Default for Scenes {
    fn default() -> Self {
        Self {
            size: 200.0,
            show_svg: true,
            always_render: false,
        }
    }
}

impl App {
    pub fn scenes(&mut self, ctx: &Context) {
        let svg = self
            .svg
            .as_mut()
            .and_then(|svg| svg.image())
            .map(|image| image.texture_id(ctx));

        egui::CentralPanel::default().show(ctx, |ui| {
            StripBuilder::new(ui)
                .sizes(Size::relative(0.5), 2)
                .horizontal(|mut strip| {
                    for kind in [SceneKind::Background, SceneKind::Foreground] {
                        strip.cell(|ui| {
                            egui::Frame::none()
                                .inner_margin(Margin::from(6.0))
                                .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
                                .show(ui, |ui| {
                                    self.scenes_header(ui, kind);
                                    self.scenes_grid(ui, kind, svg)
                                });
                        });
                    }
                });
        });
    }

    fn scenes_header(&mut self, ui: &mut Ui, kind: SceneKind) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(match kind {
                    SceneKind::Background => "Foreground",
                    SceneKind::Foreground => "Background",
                })
                .heading(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let scenes = match kind {
                    SceneKind::Background => &mut self.persistant_state.background,
                    SceneKind::Foreground => &mut self.persistant_state.foreground,
                };

                if ui
                    .add_enabled(
                        self.svg.is_some(),
                        Checkbox::new(&mut scenes.show_svg, RichText::new("SVG").heading()),
                    )
                    .changed()
                {
                    self.persistant_state.dirty = true;
                };

                if ui
                    .checkbox(
                        &mut scenes.always_render,
                        RichText::new("Render all").heading(),
                    )
                    .changed()
                {
                    self.persistant_state.dirty = true;
                };
                if ui
                    .add(
                        Slider::new(&mut scenes.size, 100.0..=512.0)
                            .show_value(false)
                            .text(RichText::new("Size").heading()),
                    )
                    .changed()
                {
                    self.persistant_state.dirty = true;
                }
            });
        });
    }

    fn scenes_grid(&mut self, ui: &mut Ui, kind: SceneKind, svg: Option<TextureId>) {
        let scenes = match kind {
            SceneKind::Background => &self.persistant_state.background,
            SceneKind::Foreground => &self.persistant_state.foreground,
        };

        egui::ScrollArea::vertical()
            .id_source(format!("{kind:?}_scroll"))
            .auto_shrink([false, false])
            .always_show_scroll(true)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 30.0);
                ui.horizontal_wrapped(|ui| {
                    crate::get_pipeline!(pipeline);
                    for (index, scene) in pipeline
                        .scenes()
                        .into_iter()
                        .filter(|(_index, scene)| scene.kind == kind)
                    {
                        ui.add_sized(
                            Vec2::new(scenes.size + 40.0, scenes.size + 60.0),
                            SceneWidget {
                                selected_scene: &mut self.selected_scene,
                                hovered_scene: &mut self.hovered_scene,

                                index,
                                scene,
                                svg: svg.filter(|_| scenes.show_svg),
                                scene_size: scenes.size,
                            },
                        );
                    }
                });
            });
    }
}

struct SceneWidget<'a> {
    selected_scene: &'a mut usize,
    hovered_scene: &'a mut usize,
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
        let res = ui.put(response.rect, Button::new("").fill(Color32::TRANSPARENT));
        if res.clicked() {
            *self.selected_scene = self.index;
        }
        if res.hovered() {
            *self.hovered_scene = self.index;
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
