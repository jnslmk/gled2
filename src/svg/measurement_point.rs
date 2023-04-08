//! Save the list of LEDs, a position for color measurement and the current color for render groups.

use super::{Led, Parameter, Svg};
use crate::texture_to_artnet::{Lamp, Positions, Universe};
use log::info;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, rc::Rc};
use usvg::{NodeExt, NodeKind, PathData, PathSegment};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MeasurementPoints {
    /// points for each render group
    points: HashMap<String, Vec<MeasurementPoint>>,
}

impl MeasurementPoints {
    pub fn positions(&self, group: &str) -> Option<Positions> {
        let points = self.points.get(group)?;
        let mut universes = HashMap::new();
        for point in points.iter() {
            let lamp = Lamp::Position {
                x: point.x,
                y: point.y,
            };

            for led in point.leds.iter() {
                if led.start % 3 != 1 {
                    panic!("Unsupported led alignment: {:?}", led);
                }
                let i = led.start / 3;

                let mut universe = universes
                    .entry(led.universe)
                    .or_insert_with(|| Universe::new(Some(led.universe)));
                universe.lamps[i] = lamp;
            }
        }

        let mut positions = Positions::default();

        assert!(universes.len() < 33);
        for (i, universe) in universes.into_values().enumerate() {
            positions.universes[i] = universe;
        }

        Some(positions)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MeasurementPoint {
    leds: Vec<Led>,
    x: f32,
    y: f32,
}

impl From<&Svg> for MeasurementPoints {
    fn from(svg: &Svg) -> Self {
        info!("find measurement points");

        let max = svg.tree.size.width().max(svg.tree.size.height()) as f32;
        let mut points = HashMap::new();
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
                                let x = rect.x() + rect.width() / 2.0;
                                let y = rect.y() + rect.height() / 2.0;
                                measurement_points.push(MeasurementPoint {
                                    leds: parameter.leds.clone(),
                                    x: x as f32 / max,
                                    y: y as f32 / max,
                                });
                            }
                        }
                    }
                });
            }
        });

        info!("Done finding measurement points");

        MeasurementPoints { points }
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
                leds: vec![parameter.leds.get(leds_added).unwrap().clone()],
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
                    leds: vec![parameter.leds.get(leds_added).unwrap().clone()],
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
                        leds: vec![parameter.leds.get(leds_added).unwrap().clone()],
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
            leds: vec![parameter.leds.get(leds_added).unwrap().clone()],
            x: prev_x as f32 / max,
            y: prev_y as f32 / max,
        });
        leds_added += 1;
    }

    assert!((path_length - path_position).abs() < 0.1);
    assert_eq!(leds_added, leds);
}
