use crate::{
    extract_output::ExtractOutput, group::Group, preview_indices::PreviewIndices,
    svg::MeasurementPoints, texture_to_output::Positions, ui::action::Action,
};
use anyhow::Result;
use egui::{mutex::Mutex, Rect};
use egui_extras::RetainedImage;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path, sync::Arc};

static MEASUREMENT_POINTS: Lazy<Arc<Mutex<MeasurementPoints>>> =
    Lazy::new(|| Arc::new(Mutex::new(MeasurementPoints::default())));

#[derive(Serialize, Deserialize)]
pub struct Svg {
    svg_contents: String,
    #[serde(skip)]
    image: Option<RetainedImage>,
}

impl std::fmt::Debug for Svg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Svg")
            .field("svg_contents", &self.svg_contents)
            .finish()
    }
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

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, &self.svg_contents)?;

        Ok(())
    }

    pub fn image(&mut self) -> Option<&RetainedImage> {
        if self.image.is_none() {
            self.image = {
                let svg = crate::svg::ParsedSvg::parse(&self.svg_contents).ok()?;
                let measurement_points = MeasurementPoints::from(&svg);
                let universes = measurement_points.universes();
                *MEASUREMENT_POINTS.lock() = measurement_points;
                let image = svg.render().ok()?;
                PreviewIndices::get().send_positions();
                *ExtractOutput::get().universes.lock() = universes;
                Action::SendPositions.enqueue();

                Some(image)
            };
        }
        self.image.as_ref()
    }
}

pub fn reset() {
    *MEASUREMENT_POINTS.lock() = MeasurementPoints::default();
}

pub fn groups() -> Vec<Group> {
    MEASUREMENT_POINTS.lock().groups()
}

pub fn positions(group: &Group) -> Positions {
    MEASUREMENT_POINTS.lock().positions(group)
}

pub fn preview_positions() -> Positions {
    MEASUREMENT_POINTS.lock().preview_positions()
}

pub fn preview_uv() -> Option<Rect> {
    MEASUREMENT_POINTS.lock().preview_uv()
}

pub fn universes() -> BTreeSet<u16> {
    MEASUREMENT_POINTS.lock().universes()
}
