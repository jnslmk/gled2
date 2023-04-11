use crate::{
    svg::{MeasurementPoints, Universes},
    texture_to_artnet::Positions,
};
use anyhow::{Context, Result};
use egui_extras::RetainedImage;
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::{Arc, RwLock};

static MEASUREMENT_POINTS: Lazy<Arc<RwLock<MeasurementPoints>>> =
    Lazy::new(|| Arc::new(RwLock::new(MeasurementPoints::default())));

pub struct Svg {
    universes: Universes,
    image: RetainedImage,
}

impl Svg {
    pub fn load(path: &Path) -> Result<Self> {
        let svg = crate::svg::Svg::read(path).context("Could not read svg file")?;
        let measurement_points = MeasurementPoints::from(&svg);
        let universes = measurement_points.universes();
        *MEASUREMENT_POINTS
            .write()
            .expect("MEASUREMENT_POINTS is poisoned") = measurement_points;
        let image = svg.render().context("Could not render svg")?;

        crate::get_pipeline!(pipeline);
        pipeline.svg_or_groups_changed(universes.clone());

        Ok(Self { universes, image })
    }

    pub fn universes(&self) -> &Universes {
        &self.universes
    }

    pub fn image(&self) -> &RetainedImage {
        &self.image
    }
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
