//! Copy a artnet buffer to the cpu and return.

use crate::{
    constants::{ARTNET_BUFFER_SIZE, UNIVERSES, UNIVERSE_BUFFER_SIZE},
    svg::Universes,
    wgpu_render_state,
};
use artnet_protocol::{ArtCommand, Output, PaddedData, PortAddress};
use wgpu::*;

#[derive(Debug)]
pub struct ExtractArtnet {
    pub output_cpu: Buffer,
}

impl ExtractArtnet {
    pub fn new() -> Self {
        let output_cpu = wgpu_render_state().device.create_buffer(&BufferDescriptor {
            size: ARTNET_BUFFER_SIZE,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            label: Some("TextureToArtnet output buffer cpu"),
            mapped_at_creation: false,
        });

        Self { output_cpu }
    }

    pub fn run(&mut self, encoder: &mut CommandEncoder, artnet: &Buffer) {
        encoder.copy_buffer_to_buffer(artnet, 0, &self.output_cpu, 0, ARTNET_BUFFER_SIZE);
    }

    pub fn poll_artnet_buffer(&mut self, universes: &Universes) -> Vec<ArtCommand> {
        let active_len = universes.len().min(UNIVERSES as usize) * UNIVERSE_BUFFER_SIZE as usize;

        if active_len == 0 {
            return vec![];
        }

        let buffer_slice = self.output_cpu.slice(..active_len as u64);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(MapMode::Read, move |v| {
            tx.send(v).expect("Could not send on oneshot sender")
        });

        // Poll the device in a blocking manner so that our future resolves.
        // In an actual application, `device.poll(...)` should
        // be called in an event loop or on another thread.
        wgpu_render_state().device.poll(Maintain::Wait);

        rx.recv()
            .expect("Could not receive on gpu rx")
            .expect("Error receiving answer to artnet_data map on gpu");

        let mut artnet_data = Vec::with_capacity(active_len);
        {
            let padded_buffer = buffer_slice.get_mapped_range();
            for chunk in padded_buffer.chunks(COPY_BYTES_PER_ROW_ALIGNMENT as usize) {
                artnet_data.extend(chunk);
            }
        }
        self.output_cpu.unmap();

        universes
            .iter()
            .take(UNIVERSES as usize)
            .zip(artnet_data.chunks(UNIVERSE_BUFFER_SIZE as usize))
            .filter_map(|(universe, data)| {
                log::debug!("Preparing artnet command for universe {universe}");
                let output = Output {
                    data: PaddedData::from(data.to_vec()),
                    port_address: PortAddress::try_from(*universe).ok()?,
                    ..Default::default()
                };

                Some(ArtCommand::Output(output))
            })
            .collect()
    }
}
