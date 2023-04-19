use crate::{pipeline::Pipeline, svg::MeasurementPoints, texture_to_artnet::Positions};
use anyhow::Result;
use egui::Rect;
use egui_extras::RetainedImage;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, RwLock},
};

static MEASUREMENT_POINTS: Lazy<Arc<RwLock<MeasurementPoints>>> =
    Lazy::new(|| Arc::new(RwLock::new(MeasurementPoints::default())));

#[derive(Serialize, Deserialize)]
pub struct Svg {
    svg_contents: String,
    #[serde(skip)]
    image: Option<RetainedImage>,
}

impl Clone for Svg {
    fn clone(&self) -> Self {
        Self {
            svg_contents: self.svg_contents.clone(),
            image: None,
        }
    }
}

impl Svg {
    pub fn load(path: &Path) -> Result<Self> {
        let svg_contents = std::fs::read_to_string(path)?;

        Ok(Self {
            svg_contents,
            image: None,
        })
    }

    pub fn image(&mut self, pipeline: &mut Pipeline) -> Option<&RetainedImage> {
        if self.image.is_none() {
            self.image = {
                let svg = crate::svg::ParsedSvg::parse(&self.svg_contents).ok()?;
                let measurement_points = MeasurementPoints::from(&svg);
                let universes = measurement_points.universes();
                *MEASUREMENT_POINTS
                    .write()
                    .expect("MEASUREMENT_POINTS is poisoned") = measurement_points;
                let image = svg.render().ok()?;
                pipeline.svg_or_groups_changed(universes);

                Some(image)
            };
        }
        self.image.as_ref()
    }
}

pub fn reset() {
    *MEASUREMENT_POINTS
        .write()
        .expect("MEASUREMENT_POINTS is poisoned") = MeasurementPoints::default();
}

pub fn groups() -> Vec<String> {
    MEASUREMENT_POINTS
        .read()
        .expect("MEASUREMENT_POINTS is poisoned")
        .groups()
}

pub fn positions(group: &str) -> Positions {
    MEASUREMENT_POINTS
        .read()
        .expect("MEASUREMENT_POINTS is poisoned")
        .positions(group)
}

pub fn preview_positions() -> Positions {
    MEASUREMENT_POINTS
        .read()
        .expect("MEASUREMENT_POINTS is poisoned")
        .preview_positions()
}

pub fn preview_uv() -> Option<Rect> {
    MEASUREMENT_POINTS
        .write()
        .expect("MEASUREMENT_POINTS is poisoned")
        .preview_uv()
}
