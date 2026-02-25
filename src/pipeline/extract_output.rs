//! Copy a output buffer to the cpu and return.

use crate::{
    OUTPUT_BUFFER,
    pipeline::constants::{UNIVERSE_BUFFER_SIZE, UNIVERSES},
    storage::asset::output_device::routing::OutputRoutings,
    svg::measurement_point::Universes,
    wgpu_render_state,
};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, bounded};
use std::{
    collections::BTreeSet,
    sync::{Arc, OnceLock},
};
use wgpu::{BufferDescriptor, BufferUsages, CommandEncoder, MapMode};

//#[derive(Clone)]
pub struct ExtractOutput {
    output_sender: Sender<Vec<u8>>,
    output_receiver: Mutex<Option<Receiver<Vec<u8>>>>,
    pub universes: Arc<Mutex<Universes>>,
    pub routings: Arc<Mutex<OutputRoutings>>,
}

impl ExtractOutput {
    pub fn get() -> &'static Self {
        static EXTRACT_OUTPUT: OnceLock<ExtractOutput> = OnceLock::new();
        EXTRACT_OUTPUT.get_or_init(|| {
            let (output_sender, output_receiver) = bounded(0);

            Self {
                output_sender,
                output_receiver: Mutex::new(Some(output_receiver)),
                universes: Arc::new(Mutex::new(BTreeSet::new())),
                routings: Arc::new(Mutex::new(OutputRoutings::default())),
            }
        })
    }

    pub fn take_output_receiver(&self) -> Option<Receiver<Vec<u8>>> {
        self.output_receiver.lock().take()
    }

    pub fn run(&self, encoder: &mut CommandEncoder) {
        let active_len = (self.universes.lock().len() as u64).min(UNIVERSES) * UNIVERSE_BUFFER_SIZE;
        if active_len > 0 {
            let buffer_desc = BufferDescriptor {
                size: active_len,
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                label: Some("TextureToOutput output buffer cpu"),
                mapped_at_creation: false,
            };
            let device = wgpu_render_state().device;
            let output_cpu = device.create_buffer(&buffer_desc);
            encoder.copy_buffer_to_buffer(&OUTPUT_BUFFER, 0, &output_cpu, 0, active_len);

            let output_sender = self.output_sender.clone();
            let capturable = output_cpu.clone();
            encoder.map_buffer_on_submit(&output_cpu, MapMode::Read, ..active_len, move |_v| {
                let output_data = capturable.get_mapped_range(..active_len).to_vec();
                output_sender
                    .send(output_data)
                    .expect("Output sender receiver lost");
                capturable.unmap();
            });
        }
    }
}
