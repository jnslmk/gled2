use egui::{Slider, Ui};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Config {
    pub center: (f32, f32),
    pub thickness: f32,
    pub count: u32,
    pub mode: u32,
    pub size: f32,
    pub common: CommonConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            center: (0.5, 0.5),
            thickness: Default::default(),
            count: Default::default(),
            size: Default::default(),
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
        data[24..28].copy_from_slice(&2f32.powi(self.common.speed_exponent).to_le_bytes());
        data[28..32].copy_from_slice(&self.size.to_le_bytes());
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

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct CommonConfig {
    #[serde(default)]
    pub direction: Direction,

    /// in 2^n of bpm
    #[serde(default)]
    pub speed_exponent: i32,
}

impl CommonConfig {
    pub fn ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            changed |= ui
                .radio_value(&mut self.direction, Direction::Forward, "Forward")
                .changed();
            changed |= ui
                .radio_value(&mut self.direction, Direction::Backward, "Backward")
                .changed();
            changed |= ui
                .radio_value(&mut self.direction, Direction::Alternating, "Alternating")
                .changed();
        });
        changed |= ui
            .add(
                Slider::new(&mut self.speed_exponent, -8..=8)
                    .step_by(1.0)
                    .custom_formatter(|n, _| {
                        format!(
                            "{} x",
                            if n > 0.0 {
                                2i32.pow(n as u32).to_string()
                            } else if n > -0.1 {
                                "1".to_string()
                            } else {
                                format!("1/{}", 2i32.pow((-n) as u32))
                            }
                        )
                    })
                    .custom_parser(|s| {
                        let s = s.split(' ').next().unwrap_or(s);
                        let s = s.strip_suffix('x').unwrap_or(s);
                        let n = if let Some(n) =
                            s.strip_prefix("1/").and_then(|s| s.parse::<f32>().ok())
                        {
                            1f32 / n
                        } else {
                            s.parse::<f32>().ok()?
                        };

                        Some(n.log2() as f64)
                    })
                    .text("Speed"),
            )
            .changed();
        ui.separator();

        ui.label("Settings");

        changed
    }
}
