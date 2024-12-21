use crate::{
    extract_output::ExtractOutput, group::Group, preview_indices::PreviewIndices,
    svg::MeasurementPoints, texture_to_output::Positions, ui::action::UiAction,
};
use anyhow::Result;
use egui::Rect;
use egui_extras::RetainedImage;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeSet, path::Path};

thread_local! {
    static MEASUREMENT_POINTS: RefCell<MeasurementPoints> = const { RefCell::new(MeasurementPoints::new()) };
}

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
                MEASUREMENT_POINTS.with_borrow_mut(|mp| {
                    *mp = measurement_points;
                });
                let image = svg.render().ok()?;
                PreviewIndices::get().send_positions();
                *ExtractOutput::get().universes.lock() = universes;
                UiAction::SendPositions.enqueue();

                Some(image)
            };
        }
        self.image.as_ref()
    }

    pub fn reset() {
        MEASUREMENT_POINTS.with_borrow_mut(|mp| {
            *mp = MeasurementPoints::new();
        });
    }

    pub fn groups() -> Vec<Group> {
        MEASUREMENT_POINTS.with_borrow(|mp| mp.groups())
    }

    pub fn positions(group: &Group) -> Positions {
        MEASUREMENT_POINTS.with_borrow(|mp| mp.positions(group))
    }

    pub fn preview_positions() -> Positions {
        MEASUREMENT_POINTS.with_borrow(|mp| mp.preview_positions())
    }

    pub fn preview_uv() -> Option<Rect> {
        MEASUREMENT_POINTS.with_borrow_mut(|mp| mp.preview_uv())
    }

    pub fn universes() -> BTreeSet<u16> {
        MEASUREMENT_POINTS.with_borrow(|mp| mp.universes())
    }
}
