use egui::{Color32, CornerRadius, Label, Rect, Vec2, Widget};
use epaint::RectShape;

use crate::ui::FRAME_STROKE;

pub struct BrightnessSlider<'a> {
    pub value: &'a mut f32,
    pub real_value: f32,
    pub size: f32,
    pub show_label: bool,
    pub horizontal: bool,
}

impl Widget for BrightnessSlider<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        const TOP_COLOR: Color32 = Color32::from_rgb(103, 103, 94);
        const HANDLE_COLOR: Color32 = Color32::from_rgb(59, 255, 0);
        const BOTTOM_COLOR: Color32 = Color32::from_rgb(255, 176, 100);

        let width = self.size;
        let mut height = ui.available_height();
        if self.show_label {
            height -= 20.0;
        }

        ui.vertical(|ui| {
            let corner_radius = 5;
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
            let handle_rect = {
                let rect = rect.shrink2(Vec2::new(0.0, corner_radius as f32));
                let (top, bottom) = rect.split_top_bottom_at_fraction(1.0 - *self.value);
                Rect::from_min_max(
                    top.left_bottom() + Vec2::new(0.0, -(corner_radius as f32)),
                    bottom.right_top() + Vec2::new(0.0, corner_radius as f32),
                )
            };
            let real_handle_rect = {
                let rect = rect.shrink2(Vec2::new(0.0, corner_radius as f32));
                let (top, bottom) = rect.split_top_bottom_at_fraction(1.0 - self.real_value);
                Rect::from_min_max(
                    top.left_bottom() + Vec2::new(0.0, -(corner_radius as f32)),
                    bottom.right_top() + Vec2::new(0.0, corner_radius as f32),
                )
            };
            let bottom = Rect::from_min_max(real_handle_rect.left_center(), rect.max);

            ui.painter()
                .add(RectShape::filled(rect.expand(2.0), 10, Color32::BLACK).with_blur_width(20.0));
            ui.painter().rect_filled(rect, corner_radius, TOP_COLOR);
            if bottom.height() > corner_radius as f32 {
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
            }
            if real_handle_rect.top() - rect.top() > corner_radius as f32 {
                ui.painter().rect_filled(
                    Rect::from_min_max(rect.min, handle_rect.max),
                    corner_radius,
                    TOP_COLOR,
                );
            }
            ui.painter()
                .rect_filled(real_handle_rect, corner_radius, BOTTOM_COLOR);
            ui.painter()
                .rect_filled(handle_rect, corner_radius, HANDLE_COLOR);
            ui.painter()
                .rect_stroke(rect, corner_radius, FRAME_STROKE, egui::StrokeKind::Inside);

            if self.show_label {
                let mut label_rect = ui.available_rect_before_wrap();
                label_rect.set_width(width);
                ui.put(
                    label_rect,
                    Label::new(format!("{:.0}%", *self.value * 100.0)),
                );
            }

            if let Some(pointer_position_2d) = response.interact_pointer_pos() {
                let relative_y = pointer_position_2d.y - rect.top();
                let new_value = 1.0 - (relative_y / rect.height()).clamp(0.0, 1.0);
                *self.value = new_value;
            }
            response
        })
        .inner
    }
}
