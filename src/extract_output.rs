//! Copy a output buffer to the cpu and return.

use crate::{
    constants::{OUTPUT_BUFFER_SIZE, UNIVERSES, UNIVERSE_BUFFER_SIZE},
    project::{OutputDevice, OutputRoutings},
    svg::Universes,
    wgpu_render_state,
};
use egui::mutex::Mutex;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};
use uuid::Uuid;
use wgpu::*;

#[derive(Clone)]
pub struct ExtractOutput {
    output_cpu: Arc<Buffer>,
    pub devices: Arc<Mutex<BTreeMap<Uuid, OutputDevice>>>,
    pub universes: Arc<Mutex<Universes>>,
    pub routings: Arc<Mutex<OutputRoutings>>,
}

impl ExtractOutput {
    pub fn get() -> &'static Self {
        static EXTRACT_OUTPUT: OnceLock<ExtractOutput> = OnceLock::new();
        EXTRACT_OUTPUT.get_or_init(|| {
            let output_cpu = wgpu_render_state().device.create_buffer(&BufferDescriptor {
                size: OUTPUT_BUFFER_SIZE,
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                label: Some("TextureToOutput output buffer cpu"),
                mapped_at_creation: false,
            });

            Self {
                output_cpu: Arc::new(output_cpu),
                devices: Arc::new(Mutex::new(BTreeMap::new())),
                universes: Arc::new(Mutex::new(BTreeSet::new())),
                routings: Arc::new(Mutex::new(OutputRoutings::default())),
            }
        })
    }

    pub fn run(&self, encoder: &mut CommandEncoder, output: &Buffer) {
        encoder.copy_buffer_to_buffer(output, 0, &self.output_cpu, 0, OUTPUT_BUFFER_SIZE);
    }

    pub fn poll_output_buffer(&self) -> Vec<u8> {
        let active_len =
            self.universes.lock().len().min(UNIVERSES as usize) * UNIVERSE_BUFFER_SIZE as usize;

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
            .expect("Error receiving answer to output_data map on gpu");

        let mut output_data = Vec::with_capacity(active_len);
        {
            let padded_buffer = buffer_slice.get_mapped_range();
            for chunk in padded_buffer.chunks(COPY_BYTES_PER_ROW_ALIGNMENT as usize) {
                output_data.extend(chunk);
            }
        }
        self.output_cpu.unmap();

        output_data
    }
}
