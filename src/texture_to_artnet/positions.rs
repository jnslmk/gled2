use crate::constants::{LAMPS_PER_UNIVERSE, POSITIONS_BUFFER_SIZE, TEXTURE_SIZE, UNIVERSES};

#[derive(Debug, Clone, Default)]
pub struct Positions {
    pub universes: [Universe; UNIVERSES as usize],
}

#[derive(Debug, Clone)]
pub struct Universe {
    pub artnet_universe: Option<u16>,
    pub lamps: [Lamp; LAMPS_PER_UNIVERSE as usize],
}

impl Default for Universe {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Universe {
    pub fn new(artnet_universe: Option<u16>) -> Self {
        Self {
            artnet_universe,
            lamps: [Lamp::default(); LAMPS_PER_UNIVERSE as usize],
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Lamp {
    #[default]
    None,
    /// Position in rendered texture
    Position { x: f32, y: f32 },
}

/// must be aligned by 16 bytes
impl From<Positions> for [u8; POSITIONS_BUFFER_SIZE as usize] {
    fn from(positions: Positions) -> Self {
        let mut data = [0u8; POSITIONS_BUFFER_SIZE as usize];
        let mut i = 0;

        for lamp in positions
            .universes
            .iter()
            .flat_map(|universe| universe.lamps.iter())
        {
            if let Lamp::Position { x, y } = lamp {
                data[i..i + 4].copy_from_slice(&x.to_le_bytes());
                data[i + 4..i + 8].copy_from_slice(&y.to_le_bytes());
            } else {
                data[i..i + 4].copy_from_slice(&2.0f32.to_le_bytes());
                data[i + 4..i + 8].copy_from_slice(&2.0f32.to_le_bytes());
            }

            i += 8;
        }

        data
    }
}

impl Lamp {
    pub fn pixel_values(&self) -> (u32, u32) {
        match self {
            Lamp::None => (0, 0),
            Lamp::Position { x, y } => (
                (x * f32::from(TEXTURE_SIZE - 1)) as u32,
                (y * f32::from(TEXTURE_SIZE - 1)) as u32,
            ),
        }
    }
}
impl PartialOrd for Lamp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Lamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.pixel_values().cmp(&other.pixel_values())
    }
}
impl PartialEq for Lamp {
    fn eq(&self, other: &Self) -> bool {
        self.pixel_values() == other.pixel_values()
    }
}
impl Eq for Lamp {}
