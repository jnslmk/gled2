use super::UNIVERSES;
use wgpu::{Buffer, Queue};

#[derive(Debug, Clone, Default)]
pub struct Positions {
    universes: [Universe; UNIVERSES as usize],
}

/// One Artnet Universe can hold 512 Positions. As we only support RGB (for now), we can have up to 170 lamps in a universe.
#[derive(Debug, Clone)]
pub struct Universe {
    lamps: [Lamp; 170],
}

impl Default for Universe {
    fn default() -> Self {
        Universe {
            lamps: [Lamp::default(); 170],
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

    pub fn write_to_buffer(&self, queue: &Queue, buffer: &Buffer) {
        queue.write_buffer(buffer, 0, &self.data());
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
