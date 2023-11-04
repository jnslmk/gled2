use egui::{RichText, Slider, Ui};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Config {
    pub center: (f32, f32),
    pub thickness: f32,
    pub count: u32,
    pub mode: u32,
    pub common: CommonConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            center: (0.5, 0.5),
            thickness: Default::default(),
            count: Default::default(),
            common: Default::default(),
            mode: Default::default(),
        }
    }
}

impl Config {
    /// must be aligned by 16 bytes
    pub fn write_data(&self, data: &mut [u8]) {
        data[0..4].copy_from_slice(&self.center.0.to_le_bytes());
        data[4..8].copy_from_slice(&self.center.1.to_le_bytes());
        data[8..12].copy_from_slice(&self.thickness.to_le_bytes());
        data[12..16].copy_from_slice(&self.count.to_le_bytes());
        data[16] = match self.common.direction {
            Direction::Forward => 0x00,
            Direction::Backward => 0x01,
            Direction::Alternating => 0x02,
        };
        data[20..24].copy_from_slice(&self.mode.to_le_bytes());
        data[24..28].copy_from_slice(&self.common.speed.to_le_bytes());
    }

    /// must be a multiple of 16
    pub const fn size() -> usize {
        16 * 2
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Forward,
    Backward,
    Alternating,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct CommonConfig {
    pub direction: Direction,

    /// in multiple of bpm
    pub speed: f32,
}

impl Default for CommonConfig {
    fn default() -> Self {
        Self {
            direction: Default::default(),
            speed: 1.0,
        }
    }
}

impl CommonConfig {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.direction, Direction::Forward, "Forward");
            ui.radio_value(&mut self.direction, Direction::Backward, "Backward");
            ui.radio_value(&mut self.direction, Direction::Alternating, "Alternating");
        });
        ui.add(
            Slider::new(&mut self.speed, 0.0..=8.0)
                .custom_formatter(|n, _| format!("{:.2} x", n))
                .text("Speed"),
        );
        ui.separator();

        ui.label(RichText::new("Settings").heading());
    }
}
