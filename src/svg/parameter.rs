//! Parameters as set in a SVG file.

use super::Led;
use crate::pipeline::{constants::LAMPS_PER_UNIVERSE, group::Group};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Parameter {
    pub start: Option<usize>,
    pub universe: u16,
    #[serde(alias = "render_groups")]
    pub groups: Vec<Group>,
    pub count: usize,
    color_channels: String,

    #[serde(skip)]
    pub leds: Vec<Led>,
}

impl Parameter {
    pub fn leds(&self) -> Vec<Led> {
        let Some(start) = self.start else {
            return vec![];
        };

        let color_channels =
            serde_json::from_str(&format!("\"{}\"", self.color_channels)).unwrap_or_default();

        { 0..self.count }
            .map(|led| {
                let mut num = start + led;
                let mut universe = self.universe;
                while num >= LAMPS_PER_UNIVERSE as usize {
                    universe += 1;
                    num -= LAMPS_PER_UNIVERSE as usize;
                }

                Led {
                    universe,
                    num,
                    color_channels,
                }
            })
            .collect()
    }
}

impl Default for Parameter {
    fn default() -> Self {
        Self {
            count: 1,
            groups: Default::default(),
            start: Default::default(),
            universe: Default::default(),
            leds: Default::default(),
            color_channels: Default::default(),
        }
    }
}
