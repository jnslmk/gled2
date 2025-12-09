pub mod multiplied_curve;
pub mod point;
pub mod polynomial;

use self::point::CurvePoint;
use super::AssetTrait;
use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2, epaint::QuadraticBezierShape};
use epaint::{CircleShape, PathShape};
use polynomial::BezierCurve;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Curve {
    linked: bool,
    points: Vec<CurvePoint>,
}
impl Eq for Curve {}

impl Default for Curve {
    fn default() -> Self {
        Self::alternating()
    }
}

impl Curve {
    pub fn alternating() -> Self {
        let points = Self::linear_points(vec![
            (0.0, 1.0),
            (1.0, 0.0),
            (2.0, 1.0),
            (3.0, 0.0),
            (4.0, 1.0),
        ]);
        Self {
            linked: true,
            points,
        }
    }

    pub fn invert_y_axis(&mut self) {
        self.points
            .iter_mut()
            .for_each(|point| point.invert_y_axis());
    }

    pub fn invert_x_axis(&mut self) {
        self.points
            .iter_mut()
            .for_each(|point| point.invert_x_axis());
    }

    fn linear_points(points: Vec<(f32, f32)>) -> Vec<CurvePoint> {
        let mut linear_points = Vec::with_capacity(2 * points.len() - 1);
        let mut prev = None;
        for (i, (x, y)) in points.iter().enumerate() {
            if let Some((prev_x, prev_y)) = prev {
                linear_points.push(CurvePoint::Bezier(Pos2::new(
                    (prev_x + x) / 2.0,
                    (prev_y + y) / 2.0,
                )));
            }

            let pos = Pos2::new(*x, *y);
            linear_points.push(if i == 0 {
                CurvePoint::First(pos)
            } else if i == points.len() - 1 {
                CurvePoint::Last(pos)
            } else {
                CurvePoint::Inner(pos)
            });

            prev = Some((x, y));
        }
        linear_points
    }

    pub fn draw(
        &mut self,
        ui: &mut Ui,
        edit_mode: bool,
        beat_progression: Option<f32>,
        rect: Rect,
    ) -> bool {
        let mut changed = false;

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(4.0, 1.0)),
            rect,
        );

        let mut outer_change = None;
        let mut bezier_to_line = None;
        let mut remove_point = None;

        let painter = ui.painter().with_clip_rect(rect);

        for i in 0..=4 {
            let stroke = Stroke::new(
                match i {
                    0 | 4 => 1.0,
                    _ => 0.5,
                },
                Color32::GRAY,
            );
            painter.add(PathShape::line(
                vec![
                    to_screen.transform_pos(Pos2::new(i as f32, 0.0)),
                    to_screen.transform_pos(Pos2::new(i as f32, 1.0)),
                ],
                stroke,
            ));
            painter.add(PathShape::line(
                vec![
                    to_screen.transform_pos(Pos2::new(0.0, i as f32 * 0.25)),
                    to_screen.transform_pos(Pos2::new(4.0, i as f32 * 0.25)),
                ],
                stroke,
            ));
        }

        if edit_mode {
            let new_point_id = ui.make_persistent_id(format!("points: {}", self.points.len()));
            let new_point_response = ui.interact(*to_screen.to(), new_point_id, Sense::click());
            if (new_point_response.double_clicked()
                || new_point_response.clicked_by(egui::PointerButton::Secondary))
                && let Some((pos, i, before, after)) =
                    new_point_response.interact_pointer_pos().and_then(|pos| {
                        self.points
                            .iter()
                            .position(|point| point.screen_pos(to_screen).x > pos.x)
                            .and_then(|i| {
                                self.points.get(i).and_then(|after| {
                                    self.points
                                        .get(i - 1)
                                        .map(|before| (pos, i, *before, *after))
                                })
                            })
                    })
            {
                if before.is_inner() || before.is_outer() {
                    self.points
                        .insert(i, CurvePoint::Inner(to_screen.inverse().transform_pos(pos)));
                    self.points.insert(
                        i,
                        CurvePoint::Bezier(to_screen.inverse().transform_pos(Pos2::new(
                            (before.screen_pos(to_screen).x + pos.x) / 2.0,
                            (before.screen_pos(to_screen).y + pos.y) / 2.0,
                        ))),
                    );
                    bezier_to_line = Some(i + 2);
                    changed = true;
                } else {
                    bezier_to_line = Some(i - 1);
                    self.points.insert(
                        i,
                        CurvePoint::Bezier(to_screen.inverse().transform_pos(Pos2::new(
                            (after.screen_pos(to_screen).x + pos.x) / 2.0,
                            (after.screen_pos(to_screen).y + pos.y) / 2.0,
                        ))),
                    );
                    self.points
                        .insert(i, CurvePoint::Inner(to_screen.inverse().transform_pos(pos)));
                    changed = true;
                }
            }

            let x_limits = { 0..self.points.len() }
                .map(|i| {
                    if i == 0 {
                        return None;
                    }
                    self.points
                        .get(i - 1)
                        .map(|point| point.screen_pos(to_screen).x)
                        .and_then(|before| {
                            self.points
                                .get(i + 1)
                                .map(|point| (before, point.screen_pos(to_screen).x))
                        })
                })
                .collect::<Vec<_>>();

            let control_point_shapes: Vec<Shape> = self
                .points
                .iter_mut()
                .enumerate()
                .map(|(i, point)| {
                    let point_id = ui.make_persistent_id(format!("point: {i}"));
                    let point_response = ui.interact(
                        point.point_rect(to_screen),
                        point_id,
                        Sense::click_and_drag(),
                    );

                    if point_response.double_clicked()
                        || point_response.clicked_by(egui::PointerButton::Secondary)
                    {
                        if point.is_outer() {
                            self.linked = !self.linked;
                            changed = true;
                            if self.linked {
                                outer_change = Some((i, point.pos()));
                            }
                        } else if point.is_bezier() {
                            bezier_to_line = Some(i);
                        } else if point.is_inner() {
                            remove_point = Some(i);
                        }
                    } else if point_response.dragged() {
                        let mut new_screen_pos =
                            point.screen_pos(to_screen) + point_response.drag_delta();
                        if let Some(x_limit) = x_limits.get(i).and_then(|x_limit| *x_limit) {
                            new_screen_pos.x = new_screen_pos.x.clamp(x_limit.0, x_limit.1)
                        }
                        point.set_screen_pos(to_screen, new_screen_pos);
                        if point_response.dragged() && point.is_outer() && self.linked {
                            outer_change = Some((i, point.pos()));
                        }
                        changed = true;
                    }

                    let stroke = if point.is_outer() && self.linked {
                        let mut stroke = ui.style().interact(&point_response).fg_stroke;
                        stroke.color = Color32::LIGHT_BLUE;
                        stroke
                    } else {
                        ui.style().interact(&point_response).fg_stroke
                    };
                    point.shape(to_screen, stroke)
                })
                .collect();

            if let Some((i, pos)) = outer_change {
                let i = self.points.len() - i - 1;
                if let Some(point) = self.points.get_mut(i) {
                    point.set_pos(pos);
                    changed = true;
                }
            }
            if let Some(i) = remove_point {
                self.points.remove(i);
                self.points.remove(i);
                bezier_to_line = Some(i - 1);
                changed = true;
            }
            if let Some(i) = bezier_to_line
                && let (Some(before), Some(after), Some(point)) = (
                    self.points.get(i - 1).map(|point| point.pos()),
                    self.points.get(i + 1).map(|point| point.pos()),
                    self.points.get_mut(i),
                )
            {
                point.set_pos(Pos2::new(
                    (before.x + after.x) / 2.0,
                    (before.y + after.y) / 2.0,
                ));
                changed = true;
            }

            painter.extend(control_point_shapes);
        }

        let points_in_screen: Vec<Pos2> = self
            .points
            .iter()
            .map(|p| p.screen_pos(to_screen))
            .collect();
        for i in 0..(points_in_screen.len() - 1) / 2 {
            let shape = QuadraticBezierShape::from_points_stroke(
                [
                    points_in_screen[i * 2],
                    points_in_screen[i * 2 + 1],
                    points_in_screen[i * 2 + 2],
                ],
                false,
                Color32::TRANSPARENT,
                Stroke::new(1.0, Color32::from_rgb(25, 200, 100)),
            );
            painter.add(shape);
        }
        if edit_mode {
            painter.add(PathShape::line(
                points_in_screen,
                Stroke::new(1.0, Color32::RED.linear_multiply(0.25)),
            ));
        }

        if let Some(beat_progression) = beat_progression {
            let beat_progression = beat_progression % 4.0;
            let color = Color32::from_rgb(0, 255, 230);
            painter.add(PathShape::line(
                vec![
                    to_screen.transform_pos(Pos2::new(beat_progression, 0.0)),
                    to_screen.transform_pos(Pos2::new(beat_progression, 1.0)),
                ],
                Stroke::new(1.0, color),
            ));

            painter.add(CircleShape::filled(
                to_screen.transform_pos(Pos2::new(
                    beat_progression,
                    1.0 - self.value(beat_progression),
                )),
                2.0,
                color,
            ));
        }

        changed
    }

    pub fn value(&self, beat_position: f32) -> f32 {
        if !(0.0..=4.0).contains(&beat_position) {
            panic!("Beat position out of range 0.0..=4.0: {beat_position}");
        }

        let value = self
            .points
            .iter()
            .position(|point| point.pos().x > beat_position)
            .and_then(|i| {
                self.points.get(i).and_then(|point| {
                    if point.is_bezier() {
                        self.points.get(i - 1).and_then(|start| {
                            self.points
                                .get(i + 1)
                                .map(|end| BezierCurve::new(start.pos(), point.pos(), end.pos()))
                        })
                    } else {
                        self.points.get(i - 1).and_then(|bezier| {
                            self.points.get(i - 2).map(|start| {
                                BezierCurve::new(start.pos(), bezier.pos(), point.pos())
                            })
                        })
                    }
                })
            })
            .map(|curve| curve.value(beat_position))
            .unwrap_or_default();

        // invert the value as the curve is upside down
        1.0 - value
    }
}

impl AssetTrait for Curve {
    const DIR_NAME: &'static str = "curves";
    const NAME: &'static str = "Curve";
    const SHOW_NAME_IF_SELECTED: bool = false;

    fn show(&self, ui: &mut egui::Ui, rect: Rect) {
        let mut min = rect.min;
        min.x = rect.max.x - 64.0;
        let rect = Rect::from_min_max(min, rect.max);

        let mut curve = self.clone();
        curve.draw(ui, false, None, rect);
    }
}
