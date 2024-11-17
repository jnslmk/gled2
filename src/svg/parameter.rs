//! Parameters as set in a SVG file.
use crate::{constants::LAMPS_PER_UNIVERSE, group::Group};

use super::Led;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Parameter {
    pub start: usize,
    pub universe: u16,
    #[serde(alias = "render_groups")]
    pub groups: Vec<Group>,
    pub count: usize,

    #[serde(skip)]
    pub leds: Vec<Led>,
}

impl Parameter {
    pub fn leds(&self) -> Vec<Led> {
        if self.start == 0 {
            return vec![];
        }

        { 0..self.count }
            .map(|led| {
                let mut num = self.start + led;
                let mut universe = self.universe;
                while num > LAMPS_PER_UNIVERSE as usize {
                    universe += 1;
                    num -= LAMPS_PER_UNIVERSE as usize;
                }

                Led { universe, num }
            })
            .collect()
    }
}

impl Default for Parameter {
    fn default() -> Self {
        Self {
            groups: vec![],
            start: 0,
            universe: 0,
            count: 1,
            leds: vec![],
        }
    }
}
