//! Copy a output buffer to the cpu and return.

use super::output_sender::OutputSender;
use crate::{
    OUTPUT_BUFFER,
    pipeline::constants::{OUTPUT_BUFFER_SIZE, UNIVERSE_BUFFER_SIZE, UNIVERSES},
    storage::asset::output_device::routing::OutputRoutings,
    svg::measurement_point::Universes,
    wgpu_render_state,
};
use egui::mutex::Mutex;
use std::{
    collections::BTreeSet,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering::Relaxed},
        mpsc::channel,
    },
};
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoder, MapMode, PollType};

static USE_FIRST_OUTPUT_BUFFER: AtomicBool = AtomicBool::new(true);

#[derive(Clone)]
pub struct ExtractOutput {
    output_cpu_0: Arc<Buffer>,
    output_cpu_1: Arc<Buffer>,
    pub universes: Arc<Mutex<Universes>>,
    pub routings: Arc<Mutex<OutputRoutings>>,
}

impl ExtractOutput {
    pub fn get() -> &'static Self {
        static EXTRACT_OUTPUT: OnceLock<ExtractOutput> = OnceLock::new();
        EXTRACT_OUTPUT.get_or_init(|| {
            let buffer_desc = BufferDescriptor {
                size: OUTPUT_BUFFER_SIZE,
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                label: Some("TextureToOutput output buffer cpu"),
                mapped_at_creation: false,
            };
            let device = wgpu_render_state().device;
            let output_cpu_0 = device.create_buffer(&buffer_desc);
            let output_cpu_1 = device.create_buffer(&buffer_desc);

            Self {
                output_cpu_0: Arc::new(output_cpu_0),
                output_cpu_1: Arc::new(output_cpu_1),
                universes: Arc::new(Mutex::new(BTreeSet::new())),
                routings: Arc::new(Mutex::new(OutputRoutings::default())),
            }
        })
    }

    pub fn trigger_output_sender(output_sender: &OutputSender) {
        output_sender
            .send(USE_FIRST_OUTPUT_BUFFER.load(Relaxed))
            .expect("Output sender receiver lost");
    }

    pub fn run(&self, encoder: &mut CommandEncoder) {
        let use_first_output_buffer = !USE_FIRST_OUTPUT_BUFFER.fetch_xor(true, Relaxed);

        encoder.copy_buffer_to_buffer(
            &OUTPUT_BUFFER,
            0,
            if use_first_output_buffer {
                &self.output_cpu_0
            } else {
                &self.output_cpu_1
            },
            0,
            OUTPUT_BUFFER_SIZE,
        );
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn poll_output_buffer(&self) -> Vec<u8> {
        let active_len =
            self.universes.lock().len().min(UNIVERSES as usize) * UNIVERSE_BUFFER_SIZE as usize;

        if active_len == 0 {
            return vec![];
        }

        let buffer = if use_first_output_buffer {
            &self.output_cpu_0
        } else {
            &self.output_cpu_1
        };
        let buffer_slice = buffer.slice(..active_len as u64);
        let (tx, rx) = channel();
        buffer_slice.map_async(MapMode::Read, move |v| {
            tx.send(v).expect("Could not send on oneshot sender")
        });

        wgpu_render_state()
            .device
            .poll(PollType::wait_indefinitely())
            .expect("Could not poll device");

        rx.recv()
            .expect("Could not receive on gpu rx")
            .expect("Error receiving answer to output_data map on gpu");

        let output_data = buffer_slice.get_mapped_range().to_vec();
        buffer.unmap();

        output_data
    }
}
