use crate::{
    shader_widget::init,
    svg::{MeasurementPoints, Universes},
};
use anyhow::{Context, Result};
use eframe::egui_wgpu::RenderState;
use egui::TextureId;
use egui_extras::RetainedImage;
use std::path::Path;

pub struct Svg {
    measurement_points: MeasurementPoints,
    universes: Universes,
    image: RetainedImage,
    texture_ids: Vec<TextureId>,
}

impl Svg {
    pub fn load(render_state: &RenderState, path: &Path) -> Result<Self> {
        let svg = crate::svg::Svg::read(path).context("Could not read svg file")?;
        let measurement_points = MeasurementPoints::from(&svg);
        let universes = measurement_points.universes();
        let image = svg.render().context("Could not render svg")?;
        let texture_ids = init(render_state, &measurement_points);

        Ok(Self {
            measurement_points,
            universes,
            image,
            texture_ids,
        })
    }

    pub fn universes(&self) -> &Universes {
        &self.universes
    }

    pub fn measurement_points(&self) -> &MeasurementPoints {
        &self.measurement_points
    }

    pub fn image(&self) -> &RetainedImage {
        &self.image
    }

    pub fn texture_ids(&self) -> &[TextureId] {
        self.texture_ids.as_ref()
    }
}
