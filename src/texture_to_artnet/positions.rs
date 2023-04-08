use super::{LAMPS_PER_UNIVERSE, POSITIONS_BUFFER_SIZE, UNIVERSES};

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
impl From<&Positions> for [u8; POSITIONS_BUFFER_SIZE as usize] {
    fn from(positions: &Positions) -> Self {
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
