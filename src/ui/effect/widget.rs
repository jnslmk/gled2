use crate::{
    pipeline::group::Groups, storage::asset::scene::effect::Effect, ui::pills::show_pills,
};
use egui::{
    Button, Color32, CornerRadius, Image, Margin, Rect, Response, Sense, Shape, TextureHandle, Ui,
    Vec2, Widget, load::SizedTexture, pos2,
};
use epaint::RectShape;

pub struct EffectWidget<'a> {
    pub selectable: Option<(&'a mut usize, usize)>,
    pub show_group: bool,
    pub effect: &'a Effect,
    pub svg: Option<TextureHandle>,
    pub groups: Option<&'a Groups>,
    pub groups_show_index: bool,
    /// used for showing the dimmer
    pub beat_progression: Option<f32>,
}

impl Widget for EffectWidget<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let frame = if let Some((selected_effect, index)) = &self.selectable {
            egui::Frame::NONE
                .fill(if *selected_effect == index {
                    Color32::GOLD
                } else {
                    Color32::TRANSPARENT
                })
                .inner_margin(Margin::from(10.0))
                .corner_radius(CornerRadius::from(4.0))
        } else {
            egui::Frame::NONE
                .inner_margin(Margin::from(10.0))
                .corner_radius(CornerRadius::from(4.0))
        };

        let response = frame
            .show(ui, |ui| {
                let mut bg_rect = ui.available_rect_before_wrap() + Margin::same(5);
                ui.painter()
                    .rect_filled(bg_rect, CornerRadius::ZERO, Color32::BLACK);
                if let Some(beat_progression) = self.beat_progression {
                    let dimmer = self.effect.opacity.value(beat_progression);
                    bg_rect.min.y += (bg_rect.height() * (1.0 - dimmer)).round().max(0.0);
                    ui.painter()
                        .rect_filled(bg_rect, CornerRadius::ZERO, Color32::DARK_GREEN);
                }

                let size = ui.available_size();
                let rect = ui.available_rect_before_wrap();
                ui.allocate_rect(rect, Sense::hover());
                ui.painter().add(Shape::Rect(RectShape::filled(
                    rect,
                    CornerRadius::default(),
                    Color32::BLACK,
                )));

                ui.painter().add(Shape::Rect({
                    let mut shape =
                        RectShape::filled(rect, CornerRadius::default(), Color32::WHITE);
                    if let Some(texture_id) = self.effect.state.texture_id() {
                        shape = shape.with_texture(
                            texture_id,
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        );
                    }
                    shape
                }));
                if let Some(svg_texture_handle) = self.svg {
                    ui.put(rect, {
                        Image::new(SizedTexture::new(svg_texture_handle.id(), size))
                    });
                }

                if self.show_group {
                    let texts = self
                        .groups
                        .as_ref()
                        .and_then(|groups| groups.get(self.effect.group_index))
                        .map(|group| {
                            vec![(
                                if self.groups_show_index {
                                    format!("{}: {}", self.effect.group_index, group.0)
                                } else {
                                    group.0.clone()
                                },
                                group.color(),
                            )]
                        })
                        .unwrap_or_else(|| {
                            vec![(self.effect.group_index.to_string(), Color32::GOLD)]
                        });
                    show_pills(ui, rect.right_top() + Vec2::new(0.0, 5.0), texts);
                }
            })
            .response;

        if let Some((selected_effect, index)) = self.selectable {
            let res = ui.put(response.rect, Button::new("").fill(Color32::TRANSPARENT));
            if res.clicked() {
                *selected_effect = index;
            }
        }

        response
    }
}
