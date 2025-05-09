use super::{ParsedSvg, color_channels::ColorChannels};
use crate::pipeline::constants::LAMPS_PER_UNIVERSE;
use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

static UNIVERSE_COLOR_CHANNELS: Lazy<Mutex<UniverseColorChannels>> =
    Lazy::new(|| Mutex::new(UniverseColorChannels::default()));

#[derive(Debug, Default)]
pub struct UniverseColorChannels {
    universes: BTreeMap<u16, [ColorChannels; LAMPS_PER_UNIVERSE as usize]>,
}

impl UniverseColorChannels {
    pub fn correct(universe: u16, data: &mut [u8]) {
        let colors_universes = UNIVERSE_COLOR_CHANNELS.lock();
        let Some(colors) = colors_universes.universes.get(&universe) else {
            return;
        };

        for (i, triple) in data.chunks_exact_mut(3).enumerate() {
            colors[i].correct(triple);
        }
    }

    pub fn set(self) {
        *UNIVERSE_COLOR_CHANNELS.lock() = self;
    }
}

impl From<&ParsedSvg> for UniverseColorChannels {
    fn from(svg: &ParsedSvg) -> Self {
        let mut universes = BTreeMap::new();
        for parameter in svg.parameters.values() {
            if parameter.start.is_none() {
                continue;
            }
            let universe = universes
                .entry(parameter.universe)
                .or_insert([ColorChannels::Rgb; LAMPS_PER_UNIVERSE as usize]);
            for led in parameter.leds() {
                universe[led.num] = led.color_channels;
            }
        }
        Self { universes }
    }
}
