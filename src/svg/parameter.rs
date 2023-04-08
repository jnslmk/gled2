//! Parameters as set in a SVG file.
use super::Led;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Parameter {
    pub start: Option<u32>,
    #[serde(alias = "render_groups")]
    pub groups: Vec<String>,
    pub count: u32,

    #[serde(skip)]
    pub leds: Vec<Led>,
}
// ANCHOR_END: parameter

impl Parameter {
    pub fn leds(&self) -> Vec<Led> {
        self.start
            .map(|start| {
                { 0..self.count }
                    .map(|led| {
                        let full_address = start + led * 3;
                        let mut universe: u16 = (full_address / 1000) as u16;
                        let mut start: usize = full_address as usize - universe as usize * 1000;
                        while start > 510 {
                            universe += 1;
                            start -= 510;
                        }

                        if start == 2 {
                            dbg!(self);
                        }

                        Led { universe, start }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for Parameter {
    fn default() -> Self {
        Self {
            groups: vec![],
            start: None,
            count: 1,
            leds: vec![],
        }
    }
}
