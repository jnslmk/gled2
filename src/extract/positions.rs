use super::{LAMPS_PER_UNIVERSE, UNIVERSES};

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

impl Positions {
    pub fn data(&self) -> Vec<u8> {
        self.universes
            .iter()
            .flat_map(|universe| universe.lamps.iter())
            .flat_map(|lamp| match lamp {
                Lamp::None => [0xff; 4].into_iter(),
                Lamp::Position { x, y } => {
                    let x = x.to_be_bytes();
                    let y = y.to_be_bytes();
                    [x[0], x[1], y[0], y[1]].into_iter()
                }
            })
            .collect::<Vec<u8>>()
    }
}

#[cfg(test)]
mod test {
    use super::Positions;
    use crate::extract::POSITIONS_BUFFER_SIZE;

    #[test]
    fn data() {
        let artnet = Positions::default();
        assert_eq!(artnet.data().len(), POSITIONS_BUFFER_SIZE as usize);
    }
}
