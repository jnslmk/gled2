//! Copy a artnet buffer to the cpu and return.

use crate::{
    svg::Universes,
    texture_to_artnet::{ARTNET_BUFFER_SIZE, UNIVERSES},
};
use artnet_protocol::{ArtCommand, Output, PortAddress};
use wgpu::*;

pub struct ExtractArtnet {
    pub output_cpu: Buffer,
}

impl ExtractArtnet {
    pub fn init(device: &Device) -> Self {
        let output_cpu = device.create_buffer(&BufferDescriptor {
            size: ARTNET_BUFFER_SIZE,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            label: Some("TextureToArtnet output buffer cpu"),
            mapped_at_creation: false,
        });

        Self { output_cpu }
    }

    pub fn run(&self, encoder: &mut CommandEncoder, artnet: &Buffer) {
        encoder.copy_buffer_to_buffer(artnet, 0, &self.output_cpu, 0, ARTNET_BUFFER_SIZE);
    }

    pub fn poll_artnet_buffer(&self, device: &Device, universes: &Universes) -> Vec<ArtCommand> {
        let active_len = universes.len().min(UNIVERSES as usize) * 512;

        let buffer_slice = self.output_cpu.slice(..active_len as u64);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(MapMode::Read, move |v| {
            tx.send(v).expect("Could not send one oneshot sender")
        });

        // Poll the device in a blocking manner so that our future resolves.
        // In an actual application, `device.poll(...)` should
        // be called in an event loop or on another thread.
        device.poll(Maintain::Wait);

        rx.recv().unwrap().unwrap();

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
            .zip(artnet_data.chunks(512))
            .filter_map(|(universe, data)| {
                let mut output = Output::from(data).ok()?;
                output.port_address = PortAddress::try_from(*universe).ok()?;

                Some(ArtCommand::Output(output))
            })
            .collect()
    }
}
