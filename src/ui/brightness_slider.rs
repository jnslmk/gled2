use egui::{Color32, CornerRadius, Label, Rect, Stroke, Vec2, Widget};
use epaint::RectShape;

pub struct BrightnessSlider<'a> {
    pub value: &'a mut f32,
    pub width: f32,
}

impl Widget for BrightnessSlider<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        const TOP_COLOR: Color32 = Color32::from_rgb(103, 103, 94);
        const HANDLE_COLOR: Color32 = Color32::from_rgb(59, 255, 0);
        const BOTTOM_COLOR: Color32 = Color32::from_rgb(255, 176, 100);

        let height = ui.available_height() - 20.0;
        ui.vertical(|ui| {
            let corner_radius = 10;
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(self.width, height),
                egui::Sense::click_and_drag(),
            );
            let handle_rect = {
                let rect = rect.shrink2(Vec2::new(0.0, corner_radius as f32));
                let (top, bottom) =
                    rect.split_top_bottom_at_fraction(1.0 - self.value.powf(1.0 / 2.0));
                Rect::from_min_max(
                    top.left_bottom() + Vec2::new(0.0, -(corner_radius as f32)),
                    bottom.right_top() + Vec2::new(0.0, corner_radius as f32),
                )
            };
            let mut draw_extra_top = false;
            let bottom = Rect::from_min_max(
                if handle_rect.top() - rect.top() > corner_radius as f32 {
                    draw_extra_top = true;
                    handle_rect.min
                } else {
                    handle_rect.left_center()
                },
                rect.max,
            );

            ui.painter().add(
                RectShape::filled(rect.expand(2.0), 10, Color32::from_white_alpha(100))
                    .with_blur_width(6.0),
            );
            ui.painter().rect_filled(rect, corner_radius, TOP_COLOR);
            ui.painter().rect_filled(
                bottom,
                CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: corner_radius,
                    se: corner_radius,
                },
                BOTTOM_COLOR,
            );
            if draw_extra_top {
                ui.painter().rect_filled(
                    Rect::from_min_max(rect.min, handle_rect.max),
                    corner_radius,
                    TOP_COLOR,
                );
            }
            ui.painter()
                .rect_filled(handle_rect, corner_radius, HANDLE_COLOR);
            ui.painter().rect_stroke(
                rect,
                corner_radius,
                Stroke {
                    width: 0.3,
                    color: Color32::WHITE,
                },
                egui::StrokeKind::Inside,
            );
            let mut label_rect = ui.available_rect_before_wrap();
            label_rect.set_width(self.width);
            ui.put(
                label_rect,
                Label::new(format!("{:.0}%", *self.value * 100.0)),
            );

            if let Some(pointer_position_2d) = response.interact_pointer_pos() {
                let relative_y = pointer_position_2d.y - rect.top();
                let new_value = 1.0 - (relative_y / rect.height()).clamp(0.0, 1.0);
                *self.value = new_value.powf(2.0);
            }
            response
        })
        .inner
    }
}
