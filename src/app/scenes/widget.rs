use crate::{animation::ColorPalette, scene::Scene};
use egui::{
    Align, Button, Checkbox, Color32, Image, Layout, Margin, Rect, Rounding, Sense, Shape, Slider,
    TextureId, Ui, Vec2, Widget,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SceneWidget<'a> {
    pub selected_scene: &'a mut usize,
    pub hovered_scene: &'a mut usize,
    pub index: usize,
    pub scene: &'a mut Scene,
    pub svg: Option<TextureId>,
    pub scenes_size: f32,
    pub live_color: Color32,
    pub uv: Option<Rect>,
}

impl<'a> Widget for SceneWidget<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let mut slider_rect = None;
        let mut checkbox_rect = None;

        let mut response = egui::Frame::none()
            .fill(if *self.selected_scene == self.index {
                Color32::GOLD.linear_multiply(
                    ((SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Could not get time")
                        .subsec_millis()
                        / 100) as f32
                        / 5.0
                        - 1.0)
                        .abs(),
                )
            } else {
                Color32::TRANSPARENT
            })
            .inner_margin(Margin::from(10.0))
            .rounding(Rounding::from(4.0))
            .show(ui, |ui| {
                egui::Frame::none()
                    .fill(if self.scene.flash {
                        Color32::WHITE
                    } else if self.scene.active {
                        let off = Color32::BLACK.to_srgba_unmultiplied();
                        let on = self.live_color.to_srgba_unmultiplied();
                        let factor = self.scene.transition_factor();

                        Color32::from_rgba_unmultiplied(
                            (off[0] as f32 * (1.0 - factor) + on[0] as f32 * factor) as u8,
                            (off[1] as f32 * (1.0 - factor) + on[1] as f32 * factor) as u8,
                            (off[2] as f32 * (1.0 - factor) + on[2] as f32 * factor) as u8,
                            (off[3] as f32 * (1.0 - factor) + on[3] as f32 * factor) as u8,
                        )
                    } else {
                        Color32::BLACK
                    })
                    .inner_margin(Margin::from(10.0))
                    .show(ui, |ui| {
                        let uv = self.uv;
                        let size = match uv {
                            Some(uv) => {
                                if uv.max.x > uv.max.y {
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
                                }
                            }
                            None => Vec2::splat(self.scenes_size),
                        };
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.set_max_width(size.x + 28.0);
                                if !self.scene.group.is_empty() {
                                    ui.add(
                                        crate::app::config::group::button(&self.scene.group, false)
                                            .sense(Sense::hover()),
                                    );
                                }
                                if let Some(hotkey) =
                                    self.scene.hotkey.as_ref().map(|key| format!("{key}"))
                                {
                                    ui.add_enabled(false, Button::new(hotkey));
                                }
                                if let Some(flash_hotkey) =
                                    self.scene.flash_hotkey.as_ref().map(|key| format!("{key}"))
                                {
                                    ui.add_enabled(
                                        false,
                                        Button::new(flash_hotkey).fill(Color32::WHITE),
                                    );
                                }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    checkbox_rect = Some(ui.checkbox(&mut false, "").rect);
                                    color_band(ui, &mut self.scene.palette);
                                });
                            });

                            ui.horizontal(|ui| {
                                let res = ui.add({
                                    let mut image = Image::new(self.scene.texture_id(), size);
                                    if let Some(uv) = uv {
                                        image = image.uv(uv);
                                    }
                                    image
                                });
                                if let Some(svg_texture_id) = self.svg {
                                    ui.put(res.rect, {
                                        let mut image = Image::new(svg_texture_id, size);
                                        if let Some(uv) = uv {
                                            image = image.uv(uv);
                                        }
                                        image
                                    });
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
            ui.add_enabled_ui(!self.scene.has_transition(), |ui| {
                let mut active = self.scene.active;
                if ui
                    .put(checkbox_rect, Checkbox::new(&mut active, ""))
                    .on_hover_text("Enable Scene")
                    .changed()
                {
                    response.mark_changed();
                }
            });
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
