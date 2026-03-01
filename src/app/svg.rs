use crate::{
    pipeline::{
        extract_output::ExtractOutput, group::Group, preview_indices::PreviewIndices,
        texture_to_output::positions::Positions,
    },
    svg::{measurement_point::MeasurementPoints, universe_color_channels::UniverseColorChannels},
    ui::action::UiAction,
};
use anyhow::Result;
use egui::{Context, TextureHandle};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeSet, path::Path, sync::Arc};

thread_local! {
    static MEASUREMENT_POINTS: RefCell<MeasurementPoints> = const { RefCell::new(MeasurementPoints::new()) };
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct Svg {
    svg_contents: String,
    #[serde(skip)]
    image: Option<TextureHandle>,
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

    pub fn image(
        &mut self,
        context: &Context,
        extract_output: &mut ExtractOutput,
    ) -> Option<TextureHandle> {
        if self.image.is_none() {
            self.image = {
                let svg = crate::svg::ParsedSvg::parse(&self.svg_contents).ok()?;
                let measurement_points = MeasurementPoints::from(&svg);
                UniverseColorChannels::from(&svg).set();
                let universes = measurement_points.universes();
                MEASUREMENT_POINTS.with_borrow_mut(|mp| {
                    *mp = measurement_points;
                });
                let image = svg.render().ok()?;
                PreviewIndices::get().send_positions();
                extract_output.universes = Arc::new(universes);
                UiAction::SendPositions.enqueue();

                let texture_handle =
                    context.load_texture("svg_preview", image, egui::TextureOptions::default());

                Some(texture_handle)
            };
        }
        self.image.clone()
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

    pub fn universes() -> BTreeSet<u16> {
        MEASUREMENT_POINTS.with_borrow(|mp| mp.universes())
    }
}
