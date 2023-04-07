pub struct Config {
    pub center: (f32, f32),
    pub thickness: f32,
    pub count: i32,
    /// Opacity: 0.0 -> 1.0
    pub opacity: f32,
    pub direction: Direction,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            center: (0.5, 0.5),
            thickness: Default::default(),
            count: Default::default(),
            opacity: 1.0,
            direction: Default::default(),
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
        data[16..20].copy_from_slice(&self.opacity.to_le_bytes());
        data[24] = match self.direction {
            Direction::Forward => 0x00,
            Direction::Backward => 0x01,
        };
    }

    /// must be a multiple of 16
    pub const fn size() -> usize {
        32
    }
}

#[derive(Default, Clone, Copy)]
pub enum Direction {
    #[default]
    Forward,
    Backward,
}
