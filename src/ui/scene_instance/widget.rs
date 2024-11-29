use crate::{
    group::Groups,
    scene_instance::SceneInstance,
    storage::{Asset, SceneInstancePath},
    ui::pills::show_pills,
};
use egui::{
    epaint::RectShape, load::SizedTexture, pos2, Align, Button, Checkbox, Color32, Image, Layout,
    Margin, Rect, Rounding, Sense, Shape, TextureId, Ui, Vec2, Widget,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SceneInstanceWidget<'a> {
    pub selected_scene_instance: &'a mut SceneInstancePath,
    pub hovered_scene_instance: &'a mut SceneInstancePath,
    pub path: SceneInstancePath,
    pub scene_instance: &'a mut SceneInstance,
    pub deck_groups: &'a Groups,
    pub svg: Option<TextureId>,
    pub effects_size: f32,
    pub live_color: Color32,
    pub uv: Option<Rect>,
}

impl<'a> Widget for SceneInstanceWidget<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let mut checkbox_rect = None;

        let mut response = egui::Frame::none()
            .fill(if *self.selected_scene_instance == self.path {
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
                    .fill(if self.scene_instance.flash {
                        Color32::WHITE
                    } else if self.scene_instance.active {
                        let off = Color32::BLACK.to_srgba_unmultiplied();
                        let on = self.live_color.to_srgba_unmultiplied();
                        let factor = self.scene_instance.transition_factor();

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
                                ui.label(
                                    Asset::get(self.scene_instance.scene)
                                        .map(|asset| asset.name().to_owned())
                                        .unwrap_or_default(),
                                );
                                if let Some(hotkey) = self
                                    .scene_instance
                                    .selection_input
                                    .as_ref()
                                    .map(|key| format!("{key}"))
                                {
                                    ui.add_enabled(false, Button::new(hotkey));
                                }
                                if let Some(flash_hotkey) = self
                                    .scene_instance
                                    .flash_input
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
                                });
                            });

                            ui.horizontal(|ui| {
                                let rect = Rect::from_min_size(ui.next_widget_position(), size);
                                ui.allocate_rect(rect, Sense::hover());
                                ui.painter().add(Shape::Rect(RectShape::filled(
                                    rect,
                                    Rounding::default(),
                                    Color32::BLACK,
                                )));

                                for texture_id in self.scene_instance.texture_ids() {
                                    ui.painter().add(Shape::Rect(RectShape {
                                        rect,
                                        rounding: Rounding::default(),
                                        blur_width: 0.0,
                                        fill_texture_id: texture_id,
                                        uv: uv.unwrap_or_else(|| {
                                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))
                                        }),
                                        fill: Color32::WHITE,
                                        stroke: Default::default(),
                                    }));
                                }

                                if let Some(svg_texture_id) = self.svg {
                                    ui.put(rect, {
                                        let mut image =
                                            Image::new(SizedTexture::new(svg_texture_id, size));
                                        if let Some(uv) = uv {
                                            image = image.uv(uv);
                                        }
                                        image
                                    });
                                }

                                let texts = self
                                    .scene_instance
                                    .group_indices()
                                    .into_iter()
                                    .filter_map(|index| {
                                        let group = self.deck_groups.get(&index)?;
                                        Some((group.0.clone(), group.color()))
                                    })
                                    .collect();
                                show_pills(ui, rect.right_top() + Vec2::new(0.0, 5.0), texts);
                            });
                        })
                    })
            })
            .response;

        let res = ui.put(response.rect, Button::new("").fill(Color32::TRANSPARENT));
        if res.clicked() {
            *self.selected_scene_instance = self.path;
        }
        if res.hovered() {
            *self.hovered_scene_instance = self.path;
        }

        if let Some(checkbox_rect) = checkbox_rect {
            ui.add_enabled_ui(!self.scene_instance.has_transition(), |ui| {
                let mut active = self.scene_instance.active;
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
