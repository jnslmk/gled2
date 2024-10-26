use crate::{
    effect::Effect,
    storage::{AssetPath, AssetTrait, Palette},
};
use egui::{
    epaint::{Vertex, WHITE_UV},
    load::SizedTexture,
    Align, Button, Checkbox, Color32, Image, Layout, Margin, Mesh, Rect, Rounding, Sense, Shape,
    Slider, TextureId, Ui, Vec2, Widget,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct EffectWidget<'a> {
    pub selected_effect: &'a mut usize,
    pub hovered_effect: &'a mut usize,
    pub index: usize,
    pub effect: &'a mut Effect,
    pub svg: Option<TextureId>,
    pub effects_size: f32,
    pub live_color: Color32,
    pub uv: Option<Rect>,
}

impl<'a> Widget for EffectWidget<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let mut slider_rect = None;
        let mut checkbox_rect = None;

        let mut response = egui::Frame::none()
            .fill(if *self.selected_effect == self.index {
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
                    .fill(if self.effect.flash {
                        Color32::WHITE
                    } else if self.effect.active {
                        let off = Color32::BLACK.to_srgba_unmultiplied();
                        let on = self.live_color.to_srgba_unmultiplied();
                        let factor = self.effect.transition_factor();

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
                            None => Vec2::splat(self.effects_size),
                        };
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.set_max_width(size.x + 28.0);
                                if !self.effect.group.is_empty() {
                                    ui.add(
                                        crate::app::config::group::button(
                                            &self.effect.group,
                                            false,
                                        )
                                        .sense(Sense::hover()),
                                    );
                                }
                                if let Some(hotkey) =
                                    self.effect.hotkey.as_ref().map(|key| format!("{key}"))
                                {
                                    ui.add_enabled(false, Button::new(hotkey));
                                }
                                if let Some(flash_hotkey) = self
                                    .effect
                                    .flash_hotkey
                                    .as_ref()
                                    .map(|key| format!("{key}"))
                                {
                                    ui.add_enabled(
                                        false,
                                        Button::new(flash_hotkey).fill(Color32::WHITE),
                                    );
                                }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    checkbox_rect = Some(ui.checkbox(&mut false, "").rect);
                                    color_band(ui, &self.effect.palette);
                                });
                            });

                            ui.horizontal(|ui| {
                                let res = ui.add({
                                    let mut image = Image::new(SizedTexture::new(
                                        self.effect.texture_id(),
                                        size,
                                    ));
                                    if let Some(uv) = uv {
                                        image = image.uv(uv);
                                    }
                                    image
                                });
                                if let Some(svg_texture_id) = self.svg {
                                    ui.put(res.rect, {
                                        let mut image =
                                            Image::new(SizedTexture::new(svg_texture_id, size));
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
            *self.selected_effect = self.index;
        }
        if res.hovered() {
            *self.hovered_effect = self.index;
        }

        if let Some(slider_rect) = slider_rect {
            ui.spacing_mut().slider_width = slider_rect.height();
            ui.put(
                slider_rect,
                Slider::new(&mut self.effect.opacity, 0.0..=1.0)
                    .vertical()
                    .show_value(false),
            )
            .on_hover_text("Effect Dimmer");
        }

        if let Some(checkbox_rect) = checkbox_rect {
            ui.add_enabled_ui(!self.effect.has_transition(), |ui| {
                let mut active = self.effect.active;
                if ui
                    .put(checkbox_rect, Checkbox::new(&mut active, ""))
                    .on_hover_text("Enable Effect")
                    .changed()
                {
                    response.mark_changed();
                }
            });
        }

        response
    }
}

fn color_band(ui: &mut Ui, path: &Option<AssetPath>) {
    let Some(path) = path.as_ref() else {
        return;
    };

    let palette = Palette::find(path).data;

    const GAP_TO_NEXT_WIDGET: f32 = 2.0;
    const COLOR_GRADIENT_RATIO: f32 = 0.35;
    const GAP: f32 = 2.0;
    const BORDER: f32 = 1.0;
    const HEIGHT: f32 = 18.0;
    let available_width = ui.available_width() - GAP - 4.0 * BORDER - GAP_TO_NEXT_WIDGET;
    let color_width = available_width * COLOR_GRADIENT_RATIO;
    let gradient_with: f32 = available_width - color_width;
    let width_per_gradient_color = gradient_with / 16.0;
    let start_pos = {
        let mut pos = ui.next_widget_position();
        pos.x -= GAP_TO_NEXT_WIDGET;
        pos
    };

    let mut mesh = Mesh::default();
    mesh.add_colored_rect(
        Rect::from_min_max(
            {
                let mut pos = start_pos;
                pos.x -= BORDER + color_width + BORDER + GAP + BORDER + gradient_with + BORDER;
                pos.y -= HEIGHT / 2.0;
                pos
            },
            {
                let mut pos = start_pos;
                pos.x -= GAP + BORDER + gradient_with + BORDER;
                pos.y += HEIGHT / 2.0;
                pos
            },
        ),
        Color32::BLACK,
    );
    let index = mesh.vertices.len() as u32;
    mesh.add_triangle(index, index + 1, index + 2);
    mesh.add_triangle(index + 3, index + 4, index + 5);
    let primary_color = palette.primary.into();

    // Top left
    mesh.vertices.push(Vertex {
        pos: {
            let mut pos = start_pos;
            pos.x -= color_width + BORDER + GAP + BORDER + gradient_with + BORDER;
            pos.y -= HEIGHT / 2.0 - BORDER;
            pos
        },
        uv: WHITE_UV,
        color: primary_color,
    });
    // Top right
    mesh.vertices.push(Vertex {
        pos: {
            let mut pos = start_pos;
            pos.x -= BORDER + GAP + BORDER + gradient_with + BORDER;
            pos.y -= HEIGHT / 2.0 - BORDER;
            pos
        },
        uv: WHITE_UV,
        color: primary_color,
    });
    // Bottom left
    mesh.vertices.push(Vertex {
        pos: {
            let mut pos = start_pos;
            pos.x -= color_width + BORDER + GAP + BORDER + gradient_with + BORDER;
            pos.y += HEIGHT / 2.0 - BORDER;
            pos
        },
        uv: WHITE_UV,
        color: primary_color,
    });
    let secondary_color = palette.secondary.into();
    // Top right
    mesh.vertices.push(Vertex {
        pos: {
            let mut pos = start_pos;
            pos.x -= BORDER + GAP + BORDER + gradient_with + BORDER;
            pos.y -= HEIGHT / 2.0 - BORDER;
            pos
        },
        uv: WHITE_UV,
        color: secondary_color,
    });
    // Bottom left
    mesh.vertices.push(Vertex {
        pos: {
            let mut pos = start_pos;
            pos.x -= color_width + BORDER + GAP + BORDER + gradient_with + BORDER;
            pos.y += HEIGHT / 2.0 - BORDER;
            pos
        },
        uv: WHITE_UV,
        color: secondary_color,
    });
    // Bottom right
    mesh.vertices.push(Vertex {
        pos: {
            let mut pos = start_pos;
            pos.x -= BORDER + GAP + BORDER + gradient_with + BORDER;
            pos.y += HEIGHT / 2.0 - BORDER;
            pos
        },
        uv: WHITE_UV,
        color: secondary_color,
    });
    // Gradient background
    mesh.add_colored_rect(
        Rect::from_min_max(
            {
                let mut pos = start_pos;
                pos.x -= BORDER + gradient_with + BORDER;
                pos.y -= HEIGHT / 2.0;
                pos
            },
            {
                let mut pos = start_pos;
                pos.y += HEIGHT / 2.0;
                pos
            },
        ),
        Color32::BLACK,
    );
    // Gradient colors
    for (i, color) in palette.gradient.into_iter().rev().enumerate() {
        mesh.add_colored_rect(
            Rect::from_min_max(
                {
                    let mut pos: egui::Pos2 = start_pos;
                    pos.x -= width_per_gradient_color * (i + 1) as f32 + BORDER;
                    pos.y -= HEIGHT / 2.0 - BORDER;
                    pos
                },
                {
                    let mut pos = start_pos;
                    pos.x -= width_per_gradient_color * i as f32 + BORDER;
                    pos.y += HEIGHT / 2.0 - BORDER;
                    pos
                },
            ),
            color.into(),
        );
    }
    ui.painter().add(Shape::mesh(mesh));
}
