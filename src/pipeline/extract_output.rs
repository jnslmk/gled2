//! Copy a output buffer to the cpu and return.

use crate::{
    OUTPUT_BUFFER,
    pipeline::constants::{OUTPUT_BUFFER_SIZE, UNIVERSE_BUFFER_SIZE, UNIVERSES},
    storage::asset::output_device::routing::OutputRoutings,
    svg::measurement_point::Universes,
    wgpu_render_state,
};
use kanal::{Receiver, Sender, bounded};
use std::sync::Arc;
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoder, MapMode, PollType, SubmissionIndex,
};

/// Number of reusable CPU-readback staging buffers kept around. Two to three
/// is enough to keep one buffer being filled on the GPU while another is mapped
/// and read on the CPU without ever allocating per frame.
const STAGING_POOL_SIZE: usize = 3;

pub struct ExtractOutput {
    output_sender: Sender<(Vec<u8>, Arc<Universes>, Arc<OutputRoutings>)>,
    /// Free-list of reusable map-read staging buffers. Buffers are lazily
    /// created on the first `run` (the wgpu device does not exist yet when
    /// `ExtractOutput::new` is called in `main`) and recycled afterwards.
    free_buffers_sender: Sender<Buffer>,
    free_buffers_receiver: Receiver<Buffer>,
    /// Submission indices of frames whose readback should be polled. The
    /// dedicated poll thread waits for each one individually so every frame's
    /// `map_buffer_on_submit` callback (the readback + network send) fires as
    /// soon as *that* frame's GPU work finishes, independently of any later
    /// frame submitted in the same displayed frame (e.g. with `double_render`).
    submit_sender: Sender<SubmissionIndex>,
    /// Held until the poll thread is started on the first `run`, then taken.
    submit_receiver: Option<Receiver<SubmissionIndex>>,
    initialized: bool,
    pub universes: Arc<Universes>,
    pub routings: Arc<OutputRoutings>,
}

impl std::fmt::Debug for ExtractOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractOutput").finish_non_exhaustive()
    }
}

impl ExtractOutput {
    pub fn new() -> (
        Self,
        Receiver<(Vec<u8>, Arc<Universes>, Arc<OutputRoutings>)>,
    ) {
        // The output channel is sized to the staging pool: at most
        // `STAGING_POOL_SIZE` readbacks can be in flight at once (each holds a
        // staging buffer until its map callback returns it), and when several
        // complete in the same device poll – e.g. when `double_render` submits
        // twice per displayed frame – every one of them must be delivered
        // instead of being collapsed into a single frame. The render thread
        // still never blocks: it `try_send`s and drops on backpressure, so a
        // genuinely overloaded output consumer simply loses the oldest surplus.
        let (output_sender, output_receiver) = bounded(STAGING_POOL_SIZE);
        let (free_buffers_sender, free_buffers_receiver) = bounded(STAGING_POOL_SIZE);
        // One slot per in-flight readback is enough: a submission is only worth
        // polling while its staging buffer is still mapped, and there are at
        // most `STAGING_POOL_SIZE` of those. Sized a little larger so that bursts
        // (double_render submitting twice per frame) are never dropped here.
        let (submit_sender, submit_receiver) = bounded(STAGING_POOL_SIZE * 2);

        (
            Self {
                output_sender,
                free_buffers_sender,
                free_buffers_receiver,
                submit_sender,
                submit_receiver: Some(submit_receiver),
                initialized: false,
                universes: Default::default(),
                routings: Default::default(),
            },
            output_receiver,
        )
    }

    /// Lazily initialise GPU-side resources once the wgpu device exists: the
    /// reusable staging buffers and the dedicated poll thread that delivers each
    /// finished frame's readback independently.
    fn ensure_initialized(&mut self) {
        if self.initialized {
            return;
        }
        let device = wgpu_render_state().device;
        for index in 0..STAGING_POOL_SIZE {
            let buffer = device.create_buffer(&BufferDescriptor {
                size: OUTPUT_BUFFER_SIZE,
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                label: Some(&format!("ExtractOutput staging buffer {index}")),
                mapped_at_creation: false,
            });
            self.free_buffers_sender
                .try_send(buffer)
                .expect("staging pool channel has capacity for the pool");
        }

        // Spawn a thread that polls the device once per submitted frame. By
        // waiting for each submission *index* in turn it lets every frame's
        // `map_buffer_on_submit` callback run the instant that frame is done on
        // the GPU – so frame A is read back and sent before frame B finishes,
        // instead of both being delivered together at the next present poll.
        if let Some(submit_receiver) = self.submit_receiver.take() {
            let poll_device = device.clone();
            std::thread::Builder::new()
                .name("gled:output:poll".to_owned())
                .spawn(move || {
                    #[cfg(feature = "profiling")]
                    profiling::register_thread!("output:poll");

                    while let Ok(index) = submit_receiver.recv() {
                        // Blocks only until this specific submission completes;
                        // the readback callback (and network send) runs here.
                        let _ = poll_device.poll(PollType::Wait {
                            submission_index: Some(index),
                            timeout: None,
                        });
                    }
                })
                .expect("Could not spawn output poll thread");
        }

        self.initialized = true;
    }

    /// Notify the poll thread that a frame has been submitted so its readback is
    /// delivered as soon as the GPU finishes it. Called by `Project::render`
    /// right after `queue.submit`. Non-blocking: if the poll thread is briefly
    /// behind, the frame still gets delivered at the next present poll.
    pub fn notify_submitted(&self, index: SubmissionIndex) {
        let _ = self.submit_sender.try_send(index);
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn run(&mut self, encoder: &mut CommandEncoder) {
        let active_len = (self.universes.len() as u64).min(UNIVERSES) * UNIVERSE_BUFFER_SIZE;
        if active_len == 0 {
            return;
        }

        self.ensure_initialized();

        // Grab a free staging buffer. If every buffer is still in flight the
        // CPU side has not kept up, so we simply skip this frame's readback –
        // the next rendered frame carries fresher data anyway.
        let Ok(Some(output_cpu)) = self.free_buffers_receiver.try_recv() else {
            return;
        };

        encoder.copy_buffer_to_buffer(&OUTPUT_BUFFER, 0, &output_cpu, 0, active_len);

        let output_sender = self.output_sender.clone();
        let free_buffers_sender = self.free_buffers_sender.clone();
        let universes = self.universes.clone();
        let routings = self.routings.clone();
        let capturable = output_cpu.clone();
        encoder.map_buffer_on_submit(&output_cpu, MapMode::Read, ..active_len, move |_v| {
            let output_data = capturable
                .get_mapped_range(..active_len)
                .expect("Could not get mapped range of output buffer")
                .to_vec();
            capturable.unmap();
            // Non-blocking: keep only the latest frame, never stall the render
            // thread on a slow output consumer.
            let _ = output_sender.try_send((output_data, universes, routings));
            // Return the buffer to the pool for reuse.
            let _ = free_buffers_sender.try_send(capturable);
        });
    }
}
