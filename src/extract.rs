//! Extract colors out of a texture into artnet buffers

mod positions;

use std::num::NonZeroU64;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

pub use positions::{Lamp, Positions, Universe};

/// One Artnet Universe can hold 512 Positions. As we only support RGB (for now), we can have up to 170 lamps in a universe.
pub const UNIVERSES: u64 = 32;
pub const LAMPS_PER_UNIVERSE: u64 = 170;
pub const LAMPS: u64 = UNIVERSES * LAMPS_PER_UNIVERSE;
pub const POSITIONS_BUFFER_SIZE: u64 = LAMPS * 4;
pub const OUTPUT_BUFFER_SIZE: u64 = UNIVERSES * 512;

pub struct Extract {
    pub pipeline: ComputePipeline,
    pub bind_group: BindGroup,
    pub positions: Buffer,
    pub output_gpu: Buffer,
    pub output_cpu: Buffer,
}

impl Extract {
    pub fn init(device: &Device, texture: &Texture, positions: &Positions) -> Self {
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("extract shader"),
            source: ShaderSource::Wgsl(include_str!("./extract/extract.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("extract bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(POSITIONS_BUFFER_SIZE),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(OUTPUT_BUFFER_SIZE),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("extract pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("extract pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "main",
        });

        let positions = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("extract positions buffer"),
            contents: &positions.data(),
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("extract texture sampler"),
            ..Default::default()
        });

        let output_gpu = device.create_buffer(&BufferDescriptor {
            size: OUTPUT_BUFFER_SIZE,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            label: Some("extract output buffer gpu"),
            mapped_at_creation: false,
        });

        let output_cpu = device.create_buffer(&BufferDescriptor {
            size: OUTPUT_BUFFER_SIZE,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            label: Some("extract output buffer cpu"),
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("extract bind group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: positions.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&texture.create_view(
                        &TextureViewDescriptor {
                            dimension: Some(TextureViewDimension::D2),
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: output_gpu.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            bind_group,
            positions,
            output_gpu,
            output_cpu,
        }
    }

    pub fn run_and_poll(&self, device: &Device, queue: &Queue) -> Vec<u8> {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("extract encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("extract compute pass"),
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);
            compute_pass.dispatch_workgroups(UNIVERSES as u32, 128, 1);
        }
        encoder.copy_buffer_to_buffer(&self.output_gpu, 0, &self.output_cpu, 0, OUTPUT_BUFFER_SIZE);

        queue.submit(Some(encoder.finish()));

        let buffer_slice = self.output_cpu.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(MapMode::Read, move |v| {
            tx.send(v).expect("Could not send one oneshot sender")
        });

        // Poll the device in a blocking manner so that our future resolves.
        // In an actual application, `device.poll(...)` should
        // be called in an event loop or on another thread.
        device.poll(Maintain::Wait);

        rx.recv().unwrap().unwrap();

        let mut output = Vec::with_capacity(OUTPUT_BUFFER_SIZE as usize);
        {
            let padded_buffer = buffer_slice.get_mapped_range();
            for chunk in padded_buffer.chunks(COPY_BYTES_PER_ROW_ALIGNMENT as usize) {
                output.extend(chunk);
            }
        }
        self.output_cpu.unmap();

        output
    }
}
