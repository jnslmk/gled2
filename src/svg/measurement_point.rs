//! Save the list of LEDs, a position for color measurement and the current color for render groups.

use super::{Led, Parameter, ParsedSvg};
use crate::{
    constants::UNIVERSES,
    texture_to_output::{Lamp, Positions, Universe},
};
use egui::{Pos2, Rect};
use kurbo::{ParamCurve, ParamCurveArclen};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tiny_skia::{Path, PathSegment, Point};
use usvg::{Group, Node};

pub type Universes = BTreeSet<u16>;

#[derive(Clone, Debug, Default)]
pub struct MeasurementPoints {
    /// points for each render group
    points: BTreeMap<String, Vec<MeasurementPoint>>,
    /// Positions of each individual led
    preview_positions: Positions,
    uv: Option<Option<Rect>>,
}

impl MeasurementPoints {
    pub fn universes(&self) -> Universes {
        self.points
            .values()
            .flat_map(|points| points.iter())
            .flat_map(|point| point.leds.iter())
            .map(|led| led.universe)
            .collect()
    }

    pub fn positions(&self, group: &str) -> Positions {
        let mut positions = Positions::default();

        if let Some(points) = self.points.get(group) {
            let mut universes = BTreeMap::new();
            for point in points.iter() {
                let lamp = Lamp::Position {
                    x: point.x,
                    y: point.y,
                };

                for led in point.leds.iter() {
                    if led.start % 3 != 1 {
                        panic!(
                            "Unsupported led alignment: {} ({:?}). supported: 1",
                            led.start % 3,
                            led
                        );
                    }
                    let i = led.start / 3;

                    let universe = universes
                        .entry(led.universe)
                        .or_insert_with(|| Universe::new(Some(led.universe)));
                    universe.lamps[i] = lamp;
                }
            }

            for (i, universe) in self
                .universes()
                .into_iter()
                .enumerate()
                .take(UNIVERSES as usize)
            {
                if let Some(universe) = universes.remove(&universe) {
                    positions.universes[i] = universe;
                }
            }
        }

        positions
    }

    pub fn preview_positions(&self) -> Positions {
        self.preview_positions.clone()
    }

    pub fn preview_uv(&mut self) -> Option<Rect> {
        *self.uv.get_or_insert_with(|| {
            self.preview_positions
                .universes
                .iter()
                .flat_map(|universe| universe.lamps.iter())
                .fold(None, |uv, lamp| match (uv, lamp) {
                    (uv, Lamp::None) => uv,
                    (None, Lamp::Position { x, y }) => {
                        Some(Rect::from_min_max(Pos2::new(*x, *y), Pos2::new(*x, *y)))
                    }
                    (Some(mut uv), Lamp::Position { x, y }) => {
                        if uv.min.x > *x {
                            uv.min.x = *x;
                        }
                        if uv.min.y > *y {
                            uv.min.y = *y;
                        }
                        if uv.max.x < *x {
                            uv.max.x = *x;
                        }
                        if uv.max.y < *y {
                            uv.max.y = *y;
                        }
                        Some(uv)
                    }
                })
                .map(|mut uv| {
                    const BORDER: f32 = 0.03;

                    uv.min.y = (uv.min.y - BORDER).max(0.0);
                    uv.min.x = (uv.min.x - BORDER).max(0.0);
                    uv.max.x = (uv.max.x + BORDER).min(1.0);
                    uv.max.y = (uv.max.y + BORDER).min(1.0);

                    uv
                })
        })
    }

    pub fn groups(&self) -> Vec<String> {
        self.points.keys().cloned().collect()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MeasurementPoint {
    leds: Vec<Led>,
    x: f32,
    y: f32,
}

fn traverse_nodes(group: &Group) -> Vec<&Node> {
    let mut nodes = vec![];
    for child in group.children() {
        nodes.push(child);
        if let Node::Group(group) = child {
            nodes.extend(traverse_nodes(group));
        }
    }
    nodes
}

impl From<&ParsedSvg> for MeasurementPoints {
    fn from(svg: &ParsedSvg) -> Self {
        debug!("find measurement points");

        let size = svg.tree.size();
        let max = size.width().max(size.height());
        let mut points = BTreeMap::new();

        traverse_nodes(svg.tree.root())
            .into_iter()
            .for_each(|node| {
                if let Some(parameter) = svg.parameters.get(&node.id().to_owned()) {
                    parameter.groups.iter().for_each(|group| {
                        let measurement_points =
                            points.entry(group.to_owned()).or_insert_with(Vec::new);
                        match node {
                            Node::Path(path) if parameter.count > 1 => {
                                let path_data = path
                                    .data()
                                    .clone()
                                    .transform(node.abs_transform())
                                    .unwrap_or_else(|| path.data().clone());
                                leds_on_path(
                                    max,
                                    parameter.count as usize,
                                    path_data,
                                    parameter,
                                    measurement_points,
                                );
                            }
                            _ => {
                                let rect = &node.abs_bounding_box();
                                let x = (rect.x() + rect.width() / 2.0) / max;
                                let y = (rect.y() + rect.height() / 2.0) / max;
                                measurement_points.push(MeasurementPoint {
                                    leds: parameter.leds.clone(),
                                    x,
                                    y,
                                });
                            }
                        }
                    });
                }
            });

        debug!("Done finding measurement points");
        debug!("Determining preview positions");

        let mut measurement_points = MeasurementPoints {
            points,
            preview_positions: Positions::default(),
            uv: None,
        };
        let universes: BTreeMap<u16, usize> = measurement_points
            .universes()
            .into_iter()
            .enumerate()
            .map(|(index, universe)| (universe, index))
            .collect();

        for measurement_point in measurement_points
            .points
            .values()
            .flat_map(|measurement_points| measurement_points.iter())
        {
            if measurement_point.leds.len() == 1 {
                let led = measurement_point
                    .leds
                    .first()
                    .expect("Could not find first led");
                if led.start % 3 != 1 {
                    panic!(
                        "Unsupported led alignment: {} ({:?}). supported: 1",
                        led.start % 3,
                        led
                    );
                }
                let i = led.start / 3;
                if let Some(universe_index) = universes.get(&led.universe) {
                    measurement_points.preview_positions.universes[*universe_index].lamps[i] =
                        Lamp::Position {
                            x: measurement_point.x,
                            y: measurement_point.y,
                        }
                }
            }
        }

        debug!("Done determining preview positions");

        measurement_points
    }
}

fn leds_on_path(
    max: f32,
    leds: usize,
    path_data: Path,
    parameter: &Parameter,
    measurement_points: &mut Vec<MeasurementPoint>,
) {
    assert_eq!(leds, parameter.leds.len());
    let path_length = path_length(&path_data) as f32;
    let led_distance = path_length / f64::from(leds as i32 - 1) as f32;
    let mut leds_added: usize = 0;
    let mut path_position: f32 = 0.0;
    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    path_data.segments().for_each(|segment| match segment {
        PathSegment::MoveTo(Point { x, y }) => {
            measurement_points.push(MeasurementPoint {
                leds: vec![parameter
                    .leds
                    .get(leds_added)
                    .expect("Could not find led")
                    .clone()],
                x: x / max,
                y: y / max,
            });
            leds_added += 1;

            prev_x = x;
            prev_y = y;
        }
        PathSegment::LineTo(Point { x, y }) => {
            let delta_x = x - prev_x;
            let delta_y = y - prev_y;
            let mut segment_position =
                led_distance * f64::from(leds_added as i32) as f32 - path_position;
            let segment_length = (delta_x.powi(2) + delta_y.powi(2)).sqrt();

            while segment_position <= segment_length {
                let x = prev_x + delta_x * segment_position / segment_length;
                let y = prev_y + delta_y * segment_position / segment_length;
                measurement_points.push(MeasurementPoint {
                    leds: vec![parameter
                        .leds
                        .get(leds_added)
                        .expect("Could not find led")
                        .clone()],
                    x: x / max,
                    y: y / max,
                });
                leds_added += 1;
                segment_position += led_distance;
            }

            prev_x = x;
            prev_y = y;
            path_position += segment_length;
        }
        PathSegment::CubicTo(Point { x: x1, y: y1 }, Point { x: x2, y: y2 }, Point { x, y }) => {
            let curve = kurbo::CubicBez::new(
                (prev_x as f64, prev_y as f64),
                (x1 as f64, y1 as f64),
                (x2 as f64, y2 as f64),
                (x as f64, y as f64),
            );
            let n = ((10.0 * curve.arclen(1.0)).ln() / 2_f64.ln()).ceil() as usize;
            let mut curves = vec![curve];
            { 0..n }.for_each(|_| {
                curves = curves
                    .iter()
                    .flat_map(|curve| {
                        let curves = curve.subdivide();
                        std::iter::once(curves.0).chain(std::iter::once(curves.1))
                    })
                    .collect();
            });
            curves.drain(..).for_each(|curve| {
                path_position += curve.arclen(1.0) as f32;
                if path_position >= led_distance * f64::from(leds_added as i32) as f32 {
                    let end = curve.end();
                    measurement_points.push(MeasurementPoint {
                        leds: vec![parameter
                            .leds
                            .get(leds_added)
                            .expect("Could not find led")
                            .clone()],
                        x: end.x as f32 / max,
                        y: end.y as f32 / max,
                    });
                    leds_added += 1;
                }
            });
            prev_x = x;
            prev_y = y;
        }
        PathSegment::QuadTo(Point { x: x1, y: y1 }, Point { x, y }) => {
            let curve = kurbo::QuadBez::new(
                kurbo::Point::new(prev_x as f64, prev_y as f64),
                kurbo::Point::new(x1 as f64, y1 as f64),
                kurbo::Point::new(x as f64, y as f64),
            );
            let n = ((10.0 * curve.arclen(1.0)).ln() / 2_f64.ln()).ceil() as usize;
            let mut curves = vec![curve];
            { 0..n }.for_each(|_| {
                curves = curves
                    .iter()
                    .flat_map(|curve| {
                        let curves = curve.subdivide();
                        std::iter::once(curves.0).chain(std::iter::once(curves.1))
                    })
                    .collect();
            });
            curves.drain(..).for_each(|curve| {
                path_position += curve.arclen(1.0) as f32;
                if path_position >= led_distance * f64::from(leds_added as i32) as f32 {
                    let end = curve.end();
                    measurement_points.push(MeasurementPoint {
                        leds: vec![parameter
                            .leds
                            .get(leds_added)
                            .expect("Could not find led")
                            .clone()],
                        x: end.x as f32 / max,
                        y: end.y as f32 / max,
                    });
                    leds_added += 1;
                }
            });
            prev_x = x;
            prev_y = y;
        }
        PathSegment::Close => {}
    });

    if leds_added == leds - 1 {
        measurement_points.push(MeasurementPoint {
            leds: vec![parameter
                .leds
                .get(leds_added)
                .expect("Could not find led")
                .clone()],
            x: prev_x / max,
            y: prev_y / max,
        });
        leds_added += 1;
    }
    assert!((path_length - path_position).abs() < 1.0);
    assert_eq!(leds_added, leds);
}

fn path_length(path: &tiny_skia::Path) -> f64 {
    let mut prev_mx = path.points()[0].x;
    let mut prev_my = path.points()[0].y;
    let mut prev_x = prev_mx;
    let mut prev_y = prev_my;

    fn create_curve_from_line(px: f32, py: f32, x: f32, y: f32) -> kurbo::CubicBez {
        let line = kurbo::Line::new(
            kurbo::Point::new(px as f64, py as f64),
            kurbo::Point::new(x as f64, y as f64),
        );
        let p1 = line.eval(0.33);
        let p2 = line.eval(0.66);
        kurbo::CubicBez::new(line.p0, p1, p2, line.p1)
    }

    let mut length = 0.0;
    for seg in path.segments() {
        let curve = match seg {
            tiny_skia::PathSegment::MoveTo(p) => {
                prev_mx = p.x;
                prev_my = p.y;
                prev_x = p.x;
                prev_y = p.y;
                continue;
            }
            tiny_skia::PathSegment::LineTo(p) => create_curve_from_line(prev_x, prev_y, p.x, p.y),
            tiny_skia::PathSegment::QuadTo(p1, p) => kurbo::QuadBez::new(
                kurbo::Point::new(prev_x as f64, prev_y as f64),
                kurbo::Point::new(p1.x as f64, p1.y as f64),
                kurbo::Point::new(p.x as f64, p.y as f64),
            )
            .raise(),
            tiny_skia::PathSegment::CubicTo(p1, p2, p) => kurbo::CubicBez::new(
                kurbo::Point::new(prev_x as f64, prev_y as f64),
                kurbo::Point::new(p1.x as f64, p1.y as f64),
                kurbo::Point::new(p2.x as f64, p2.y as f64),
                kurbo::Point::new(p.x as f64, p.y as f64),
            ),
            tiny_skia::PathSegment::Close => {
                create_curve_from_line(prev_x, prev_y, prev_mx, prev_my)
            }
        };

        length += curve.arclen(0.5);
        prev_x = curve.p3.x as f32;
        prev_y = curve.p3.y as f32;
    }

    length
}
