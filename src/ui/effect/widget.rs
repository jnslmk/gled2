use crate::{
    storage::asset::scene::{effect::Effect, effect_state::EffectState},
    ui::pills::show_pills,
};
use egui::{
    Button, Color32, CornerRadius, Margin, Rect, Response, Sense, Shape, Ui, Vec2, Widget, pos2,
};
use epaint::RectShape;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct EffectWidget<'a> {
    pub selectable: Option<(&'a mut usize, usize)>,
    pub show_group: bool,
    pub effect: &'a Effect,
    pub effect_state: &'a EffectState,
}

impl Widget for EffectWidget<'_> {
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
                .corner_radius(CornerRadius::from(4.0))
        } else {
            egui::Frame::none()
                .inner_margin(Margin::from(10.0))
                .corner_radius(CornerRadius::from(4.0))
        };

        let response = frame
            .show(ui, |ui| {
                let rect = ui.available_rect_before_wrap();
                ui.allocate_rect(rect, Sense::hover());
                ui.painter().add(Shape::Rect(RectShape::filled(
                    rect,
                    CornerRadius::default(),
                    Color32::BLACK,
                )));
                ui.painter().add(Shape::Rect(
                    RectShape::filled(rect, CornerRadius::default(), Color32::WHITE).with_texture(
                        self.effect_state.texture_id(),
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    ),
                ));

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
