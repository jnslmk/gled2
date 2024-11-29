use crate::{
    effect::{Effect, EffectState},
    ui::pills::show_pills,
};
use egui::{
    pos2, Button, Color32, Margin, Rect, Response, Rounding, Sense, Shape, Ui, Vec2, Widget,
};
use epaint::RectShape;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct EffectWidget<'a> {
    pub selectable: Option<(&'a mut usize, usize)>,
    pub show_group: bool,
    pub effect: &'a Effect,
    pub effect_state: &'a EffectState,
}

impl<'a> Widget for EffectWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let frame = if let Some((selected_effect, index)) = &self.selectable {
            egui::Frame::none()
                .fill(if *selected_effect == index {
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
        } else {
            egui::Frame::none()
                .inner_margin(Margin::from(10.0))
                .rounding(Rounding::from(4.0))
        };

        let response = frame
            .show(ui, |ui| {
                let rect = ui.available_rect_before_wrap();
                ui.allocate_rect(rect, Sense::hover());
                ui.painter().add(Shape::Rect(RectShape::filled(
                    rect,
                    Rounding::default(),
                    Color32::BLACK,
                )));
                ui.painter().add(Shape::Rect(RectShape {
                    rect,
                    rounding: Rounding::default(),
                    blur_width: 0.0,
                    fill_texture_id: self.effect_state.texture_id(),
                    uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    fill: Color32::WHITE,
                    stroke: Default::default(),
                }));

                if self.show_group {
                    show_pills(
                        ui,
                        rect.right_top() + Vec2::new(0.0, 5.0),
                        vec![(self.effect.group_index.to_string(), Color32::GOLD)],
                    );
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
