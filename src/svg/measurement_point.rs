//! Save the list of LEDs, a position for color measurement and the current color for render groups.

use super::{Led, Parameter, ParsedSvg};
use crate::{
    constants::UNIVERSES,
    texture_to_artnet::{Lamp, Positions, Universe},
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use usvg::{NodeExt, NodeKind, PathData, PathSegment};

pub type Universes = BTreeSet<u16>;

#[derive(Clone, Debug, Default)]
pub struct MeasurementPoints {
    /// points for each render group
    points: BTreeMap<String, Vec<MeasurementPoint>>,
    /// Positions of each individual led
    preview_positions: Positions,
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
                        panic!("Unsupported led alignment: {:?}. supported: 1", led);
                    }
                    let i = led.start / 3;

                    let mut universe = universes
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

impl From<&ParsedSvg> for MeasurementPoints {
    fn from(svg: &ParsedSvg) -> Self {
        debug!("find measurement points");

        let max = svg.tree.size.width().max(svg.tree.size.height()) as f32;
        let mut points = BTreeMap::new();
        svg.tree.root.descendants().for_each(|node| {
            if let Some(parameter) = svg.parameters.get(&node.borrow().id().to_owned()) {
                parameter.groups.iter().for_each(|group| {
                    let measurement_points =
                        points.entry(group.to_owned()).or_insert_with(Vec::new);
                    match &*node.borrow() {
                        NodeKind::Path(path) if parameter.count > 1 => {
                            let mut path_data = Rc::clone(&path.data);
                            Rc::make_mut(&mut path_data).transform(node.abs_transform());
                            leds_on_path(
                                max,
                                parameter.count as usize,
                                path_data,
                                parameter,
                                measurement_points,
                            );
                        }
                        _ => {
                            if let Some(rect) = &node.calculate_bbox() {
                                let x = (rect.x() + rect.width() / 2.0) as f32 / max;
                                let y = (rect.y() + rect.height() / 2.0) as f32 / max;
                                measurement_points.push(MeasurementPoint {
                                    leds: parameter.leds.clone(),
                                    x,
                                    y,
                                });
                            }
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
                    .get(0)
                    .expect("Could not find first led");
                if led.start % 3 != 1 {
                    panic!("Unsupported led alignment: {:?}. supported: 1", led);
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
    path_data: Rc<PathData>,
    parameter: &Parameter,
    measurement_points: &mut Vec<MeasurementPoint>,
) {
    assert_eq!(leds, parameter.leds.len());
    let path_length = path_data.length();
    let led_distance = path_length / f64::from(leds as i32 - 1);
    let mut leds_added: usize = 0;
    let mut path_position = 0.0;
    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    path_data.segments().for_each(|segment| match segment {
        PathSegment::MoveTo { x, y } => {
            measurement_points.push(MeasurementPoint {
                leds: vec![parameter
                    .leds
                    .get(leds_added)
                    .expect("Could not find led")
                    .clone()],
                x: x as f32 / max,
                y: y as f32 / max,
            });
            leds_added += 1;

            prev_x = x;
            prev_y = y;
        }
        PathSegment::LineTo { x, y } => {
            let delta_x = x - prev_x;
            let delta_y = y - prev_y;
            let mut segment_position =
                led_distance + f64::from(leds_added as i32 - 1) * led_distance - path_position;
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
                    x: x as f32 / max,
                    y: y as f32 / max,
                });
                leds_added += 1;
                segment_position += led_distance;
            }

            prev_x = x;
            prev_y = y;
            path_position += segment_length;
        }
        PathSegment::CurveTo {
            x1,
            y1,
            x2,
            y2,
            x,
            y,
        } => {
            use kurbo::{ParamCurve, ParamCurveArclen};
            let curve = kurbo::CubicBez::new((prev_x, prev_y), (x1, y1), (x2, y2), (x, y));
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
                path_position += curve.arclen(1.0);
                if path_position >= led_distance * f64::from(leds_added as i32) {
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
        PathSegment::ClosePath => {}
    });

    if leds_added == leds - 1 {
        measurement_points.push(MeasurementPoint {
            leds: vec![parameter
                .leds
                .get(leds_added)
                .expect("Could not find led")
                .clone()],
            x: prev_x as f32 / max,
            y: prev_y as f32 / max,
        });
        leds_added += 1;
    }

    assert!((path_length - path_position).abs() < 0.1);
    assert_eq!(leds_added, leds);
}
