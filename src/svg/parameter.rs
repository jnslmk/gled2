//! Parameters as set in a SVG file.
use super::Led;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Parameter {
    pub start: u32,
    pub universe: u16,
    #[serde(alias = "render_groups")]
    pub groups: Vec<String>,
    pub count: u32,

    #[serde(skip)]
    pub leds: Vec<Led>,
}

impl Parameter {
    pub fn leds(&self) -> Vec<Led> {
        { 0..self.count }
            .map(|led| {
                let full_address = self.start + led * 3;
                let mut universe: u16 = self.universe;
                let mut start: usize = full_address as usize;
                dbg!("bla");
                dbg!(start);
                dbg!(universe);
                while start > 510 {
                    universe += 1;
                    start -= 510;
                }
                dbg!(start);
                dbg!(universe);

                Led { universe, start }
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
