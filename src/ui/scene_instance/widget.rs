use crate::{
    app::timing::Timing,
    pipeline::{constants::PREVIEW_TEXTURE_SIZE, group::Groups},
    storage::asset::{
        Asset,
        project::{DeckPath, scene_instance_path::SceneInstancePathId},
        scene::instance::SceneInstance,
    },
    ui::{FRAME_STROKE, gled_slider::GledSlider, pills::show_pills},
};
use egui::{
    Align, Button, Checkbox, Color32, ColorImage, Context, CornerRadius, Image, Label, Layout,
    Margin, Rect, Sense, Shadow, Shape, TextureHandle, TextureId, TextureOptions, Ui, Vec2, Widget,
    epaint::RectShape, load::SizedTexture, pos2,
};
use once_cell::sync::OnceCell;
use usvg::Tree;

static SELECTED_SVG: &str = include_str!("selected.svg");
static SELECTED_IMAGE: OnceCell<TextureHandle> = OnceCell::new();
fn selected_image(ctx: &Context) -> TextureId {
    SELECTED_IMAGE
        .get_or_init(|| {
            let tree = Tree::from_str(SELECTED_SVG, &Default::default()).unwrap();
            let mut pixmap =
                tiny_skia::Pixmap::new(PREVIEW_TEXTURE_SIZE as u32, PREVIEW_TEXTURE_SIZE as u32)
                    .unwrap();
            resvg::render(
                &tree,
                tiny_skia::Transform::from_scale(
                    f32::from(PREVIEW_TEXTURE_SIZE) / tree.size().width(),
                    f32::from(PREVIEW_TEXTURE_SIZE) / tree.size().height(),
                ),
                &mut pixmap.as_mut(),
            );
            let image = ColorImage::from_rgba_unmultiplied(
                [PREVIEW_TEXTURE_SIZE as usize; 2],
                pixmap.data(),
            );
            ctx.load_texture("selected_scene_image", image, TextureOptions::default())
        })
        .id()
}

pub struct SceneInstanceWidget<'a> {
    pub selected_scene_instance: &'a mut SceneInstancePathId,
    pub deck_path: DeckPath,
    pub scene_instance: &'a mut SceneInstance,
    pub groups: &'a Groups,
    pub dnd_handle: egui_dnd::Handle<'a>,
    pub svg: Option<TextureHandle>,
    pub size: Vec2,
    pub timing: &'a Timing,
}

impl Widget for SceneInstanceWidget<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let mut checkbox_rect = None;
        let path = SceneInstancePathId {
            deck_path: self.deck_path,
            id: self.scene_instance.id,
        };

        if *self.selected_scene_instance == path {
            ui.painter().image(
                selected_image(ui.ctx()),
                ui.available_rect_before_wrap(),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        let mut response = egui::Frame::NONE
            .inner_margin(Margin::from(10.0))
            .corner_radius(CornerRadius::from(4.0))
            .show(ui, |ui| {
                egui::Frame::NONE
                    .inner_margin(Margin::from(10.0))
                    .stroke(FRAME_STROKE)
                    .shadow(if self.scene_instance.active {
                        Shadow {
                            offset: [0, 0],
                            blur: 10,
                            spread: 4,
                            color: Color32::from_rgba_unmultiplied_const(255, 255, 255, 128),
                        }
                    } else {
                        Shadow::NONE
                    })
                    .show(ui, |ui| {
                        let bg_rect = Rect::from_min_size(ui.cursor().min, self.size)
                            + Margin {
                                left: 10,
                                right: 30,
                                top: 10,
                                bottom: 30,
                            };

                        if self.scene_instance.active {
                            ui.painter().rect_filled(
                                bg_rect,
                                CornerRadius::ZERO,
                                self.scene_instance.color,
                            );
                        }

                        let mut beat_progression = self.timing.beat_progression();
                        beat_progression += self
                            .scene_instance
                            .beat_progression_offset
                            .value(beat_progression);
                        let dimmer = if self.scene_instance.active {
                            self.scene_instance.transition_factor()
                        } else {
                            1.0
                        } * self.scene_instance.input_dimmer
                            * self.scene_instance.opacity.value(beat_progression);

                        if self.scene_instance.flash {
                            ui.painter().rect_filled(
                                bg_rect.shrink(1.0),
                                CornerRadius::ZERO,
                                Color32::from_white_alpha(180),
                            );
                        } else {
                            ui.painter().rect_filled(
                                bg_rect.shrink(1.0),
                                CornerRadius::ZERO,
                                self.scene_instance.color,
                            );
                        }

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    Vec2::new(ui.available_width() - 10.0, 20.0),
                                    Label::new(
                                        Asset::get(self.scene_instance.scene)
                                            .map(|asset| asset.name().to_owned())
                                            .unwrap_or_default(),
                                    )
                                    .truncate(),
                                );

                                //ui.add_space(10.0);

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    checkbox_rect = Some(ui.checkbox(&mut false, "").rect);
                                });
                            });

                            self.dnd_handle.ui(ui, |ui| {
                                ui.scope(|ui| {
                                    let rect =
                                        Rect::from_min_size(ui.next_widget_position(), self.size);
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
                                                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                            ),
                                        ));
                                    }

                                    if let Some(svg_texture_handle) = self.svg {
                                        ui.put(rect, {
                                            Image::new(SizedTexture::new(
                                                svg_texture_handle.id(),
                                                self.size,
                                            ))
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

                                    show_pills(ui, rect.right_top() + Vec2::new(0.0, 5.0), texts);

                                    if !self.scene_instance.effect_overwrites.is_empty()
                                        || self.scene_instance.groups_overwrite.is_some()
                                        || self.scene_instance.palette_overwrite.is_some()
                                    {
                                        ui.painter().text(
                                            rect.left_bottom() + Vec2::new(2.0, -1.0),
                                            egui::Align2::LEFT_BOTTOM,
                                            "⚙",
                                            egui::TextStyle::Body.resolve(ui.style()),
                                            Color32::from_white_alpha(100),
                                        );
                                    }

                                    ui.place(
                                        Rect::from_min_size(
                                            rect.right_top() + Vec2::new(10.0, 0.0),
                                            Vec2::new(10.0, rect.height()),
                                        ),
                                        GledSlider {
                                            real_value: if self.scene_instance.active {
                                                dimmer
                                            } else {
                                                0.0
                                            },
                                            size: 10.0,
                                            max_value: 100.0,
                                            value: &mut self.scene_instance.opacity.multiplier(),
                                            show_label: false,
                                            horizontal: false,
                                        },
                                    );
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
