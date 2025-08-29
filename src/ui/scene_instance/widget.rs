use crate::{
    pipeline::group::Groups,
    storage::asset::{
        Asset,
        project::{DeckPath, scene_instance_path::SceneInstancePathId},
        scene::instance::SceneInstance,
    },
    ui::pills::show_pills,
};
use egui::{
    Align, Button, Checkbox, Color32, CornerRadius, Image, Label, Layout, Margin, Rect, Sense,
    Shape, TextureHandle, Ui, Vec2, Widget, epaint::RectShape, load::SizedTexture, pos2,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SceneInstanceWidget<'a> {
    pub selected_scene_instance: &'a mut SceneInstancePathId,
    pub deck_path: DeckPath,
    pub scene_instance: &'a mut SceneInstance,
    pub groups: &'a Groups,
    pub dnd_handle: egui_dnd::Handle<'a>,
    pub svg: Option<TextureHandle>,
    pub effects_size: f32,
    pub live_color: Color32,
    pub uv: Option<Rect>,
}

impl Widget for SceneInstanceWidget<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let mut checkbox_rect = None;
        let path = SceneInstancePathId {
            deck_path: self.deck_path,
            id: self.scene_instance.id,
        };

        let mut response = egui::Frame::NONE
            .fill(if *self.selected_scene_instance == path {
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
            .corner_radius(CornerRadius::from(4.0))
            .show(ui, |ui| {
                egui::Frame::NONE
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
                                ui.set_max_width(size.x);
                                ui.add(
                                    Label::new(
                                        Asset::get(self.scene_instance.scene)
                                            .map(|asset| asset.name().to_owned())
                                            .unwrap_or_default(),
                                    )
                                    .truncate(),
                                );

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    checkbox_rect = Some(ui.checkbox(&mut false, "").rect);
                                });
                            });

                            self.dnd_handle.ui(ui, |ui| {
                                ui.scope(|ui| {
                                    let rect = Rect::from_min_size(ui.next_widget_position(), size);
                                    ui.allocate_rect(rect, Sense::hover());
                                    ui.painter().add(Shape::Rect(RectShape::filled(
                                        rect,
                                        CornerRadius::default(),
                                        Color32::BLACK,
                                    )));

                                    for texture_id in self.scene_instance.texture_ids() {
                                        ui.painter().add(Shape::Rect(
                                            RectShape::filled(
                                                rect,
                                                CornerRadius::default(),
                                                Color32::WHITE,
                                            )
                                            .with_texture(
                                                texture_id,
                                                uv.unwrap_or_else(|| {
                                                    Rect::from_min_max(
                                                        pos2(0.0, 0.0),
                                                        pos2(1.0, 1.0),
                                                    )
                                                }),
                                            ),
                                        ));
                                    }

                                    if let Some(svg_texture_handle) = self.svg {
                                        ui.put(rect, {
                                            let mut image = Image::new(SizedTexture::new(
                                                svg_texture_handle.id(),
                                                size,
                                            ));
                                            if let Some(uv) = uv {
                                                image = image.uv(uv);
                                            }
                                            image
                                        });
                                    }

                                    let groups = self
                                        .scene_instance
                                        .groups_overwrite
                                        .as_ref()
                                        .unwrap_or(self.groups);

                                    let texts = self
                                        .scene_instance
                                        .group_indices()
                                        .into_iter()
                                        .filter_map(|index| {
                                            let group = groups.get(index)?;
                                            Some((group.0.clone(), group.color()))
                                        })
                                        .collect();
                                    ui.set_clip_rect(rect);
                                    show_pills(ui, rect.right_top() + Vec2::new(0.0, 5.0), texts);
                                });
                            })
                        })
                    })
            })
            .response;

        let res = ui.put(response.rect, Button::new("").fill(Color32::TRANSPARENT));
        if res.clicked() {
            *self.selected_scene_instance = path;
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
