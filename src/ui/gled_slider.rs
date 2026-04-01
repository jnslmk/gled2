use egui::{Color32, Label, Rect, Vec2, Widget};
use epaint::RectShape;

use crate::ui::FRAME_STROKE;

pub struct GledSlider<'a> {
    value: &'a mut f32,
    preview_value: Option<f32>,
    max_value: f32,
    size: f32,
    show_label: bool,
    horizontal: bool,
}

impl<'a> GledSlider<'a> {
    pub fn new(value: &'a mut f32, max_value: f32) -> Self {
        Self {
            value,
            preview_value: None,
            max_value,
            size: 50.0,
            show_label: false,
            horizontal: false,
        }
    }

    pub fn preview_value(mut self, preview_value: f32) -> Self {
        self.preview_value = Some(preview_value);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    pub fn show_label(mut self) -> Self {
        self.show_label = true;
        self
    }
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }
}

impl Widget for GledSlider<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        const TOP_COLOR: Color32 = Color32::from_rgb(103, 103, 94);
        const HANDLE_COLOR: Color32 = Color32::from_rgb(59, 255, 0);
        const BOTTOM_COLOR: Color32 = Color32::from_rgb(255, 176, 100);

        let preview_value = self.preview_value.unwrap_or(*self.value);

        if self.horizontal {
            let height: f32 = self.size;
            let mut width = ui.available_width();
            if self.show_label {
                width -= 20.0;
            }

            ui.horizontal(|ui| {
                let corner_radius = 5;
                let (rect, mut response) = ui
                    .allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
                let (left, right) = rect.split_left_right_at_fraction(preview_value);
                let handle_rect = {
                    let rect = rect.shrink2(Vec2::new(corner_radius as f32, 0.0));
                    let (left, _right) = rect.split_left_right_at_fraction(*self.value);
                    Rect::from_min_max(
                        left.right_top() + Vec2::new(-(corner_radius as f32), 0.0),
                        left.right_bottom() + Vec2::new(corner_radius as f32, 0.0),
                    )
                };

                ui.painter_at(left)
                    .rect_filled(rect, corner_radius, BOTTOM_COLOR);
                ui.painter_at(right)
                    .rect_filled(rect, corner_radius, TOP_COLOR);
                ui.painter()
                    .rect_filled(handle_rect, corner_radius, HANDLE_COLOR);
                ui.painter().rect_stroke(
                    rect,
                    corner_radius,
                    FRAME_STROKE,
                    egui::StrokeKind::Inside,
                );

                if self.show_label {
                    let mut label_rect = ui.available_rect_before_wrap();
                    label_rect.set_height(height);
                    ui.put(
                        label_rect,
                        Label::new(format!("{:.0}%", *self.value * self.max_value)),
                    );
                }

                if let Some(pointer_position_2d) = response.interact_pointer_pos() {
                    let new_value =
                        ((pointer_position_2d.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                    *self.value = new_value;
                    response.mark_changed();
                }
                response
            })
            .inner
        } else {
            let width = self.size;
            let mut height = ui.available_height();
            if self.show_label {
                height -= 20.0;
            }

            ui.vertical(|ui| {
                let corner_radius = 5;
                let (rect, mut response) = ui
                    .allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
                let (top, bottom) = rect.split_top_bottom_at_fraction(1.0 - preview_value);
                let handle_rect = {
                    let rect = rect.shrink2(Vec2::new(0.0, corner_radius as f32));
                    let (top, _bottom) = rect.split_top_bottom_at_fraction(1.0 - *self.value);
                    Rect::from_min_max(
                        top.left_bottom() + Vec2::new(0.0, -(corner_radius as f32)),
                        top.right_bottom() + Vec2::new(0.0, corner_radius as f32),
                    )
                };

                ui.painter().add(
                    RectShape::filled(
                        rect.expand(2.0),
                        corner_radius,
                        Color32::from_black_alpha(60),
                    )
                    .with_blur_width(6.0),
                );
                ui.painter_at(top)
                    .rect_filled(rect, corner_radius, TOP_COLOR);
                ui.painter_at(bottom)
                    .rect_filled(rect, corner_radius, BOTTOM_COLOR);
                ui.painter()
                    .rect_filled(handle_rect, corner_radius, HANDLE_COLOR);
                ui.painter().rect_stroke(
                    rect,
                    corner_radius,
                    FRAME_STROKE,
                    egui::StrokeKind::Inside,
                );

                if self.show_label {
                    let mut label_rect = ui.available_rect_before_wrap();
                    label_rect.set_width(width);
                    ui.put(
                        label_rect,
                        Label::new(format!("{:.0}%", *self.value * 100.0)),
                    );
                }

                if let Some(pointer_position_2d) = response.interact_pointer_pos() {
                    let new_value = 1.0
                        - ((pointer_position_2d.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
                    *self.value = new_value;
                    response.mark_changed();
                }
                response
            })
            .inner
        }
    }
}
