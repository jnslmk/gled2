use super::{LAMPS_PER_UNIVERSE, POSITIONS_BUFFER_SIZE, UNIVERSES};

#[derive(Debug, Clone, Default)]
pub struct Positions {
    pub universes: [Universe; UNIVERSES as usize],
}

#[derive(Debug, Clone)]
pub struct Universe {
    pub lamps: [Lamp; LAMPS_PER_UNIVERSE as usize],
}

impl Default for Universe {
    fn default() -> Self {
        Universe {
            lamps: [Lamp::default(); LAMPS_PER_UNIVERSE as usize],
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Lamp {
    #[default]
    None,
    /// Position in rendered texture
    Position { x: u16, y: u16 },
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
                let x = x.to_le_bytes();
                let y = y.to_le_bytes();
                data[i] = x[0];
                data[i + 1] = x[1];
                data[i + 2] = y[0];
                data[i + 3] = y[1];
            }

            i += 4;
        }

        data
    }
}
