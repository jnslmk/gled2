use crate::{
    shader_widget::init,
    svg::{MeasurementPoints, Universes},
    texture_to_artnet::Positions,
};
use anyhow::{Context, Result};
use egui::TextureId;
use egui_extras::RetainedImage;
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::{Arc, RwLock};

static MEASUREMENT_POINTS: Lazy<Arc<RwLock<MeasurementPoints>>> =
    Lazy::new(|| Arc::new(RwLock::new(MeasurementPoints::default())));

pub struct Svg {
    universes: Universes,
    image: RetainedImage,
    texture_ids: Vec<TextureId>,
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
        let texture_ids = init();

        Ok(Self {
            universes,
            image,
            texture_ids,
        })
    }

    pub fn universes(&self) -> &Universes {
        &self.universes
    }

    pub fn image(&self) -> &RetainedImage {
        &self.image
    }

    pub fn texture_ids(&self) -> &[TextureId] {
        self.texture_ids.as_ref()
    }
}

pub fn positions(group: &str) -> Option<Positions> {
    MEASUREMENT_POINTS.read().ok()?.positions(group)
}
