//! Combine two output buffers into one output buffer.
use crate::{OUTPUT_BUFFER, pipeline::constants::OUTPUT_BUFFER_SIZE, wgpu_render_state};
use std::num::NonZeroU64;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, CommandEncoder,
    ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, PipelineLayoutDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages,
};

#[derive(Debug, PartialEq)]
pub struct OutputMix {
    pipeline: ComputePipeline,
    bind_group: BindGroup,
}

impl OutputMix {
    pub fn new(buffer: &Buffer) -> Self {
        let device = wgpu_render_state().device;

        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("OutputMix shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/output_mix.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("OutputMix bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(OUTPUT_BUFFER_SIZE),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(OUTPUT_BUFFER_SIZE),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("OutputMix pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            cache: None,
            label: Some("OutputMix pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("OutputMix bind group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: OUTPUT_BUFFER.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            bind_group,
        }
    }

    pub fn run(&self, encoder: &mut CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("OutputMix compute pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(OUTPUT_BUFFER_SIZE as u32 / 4, 1, 1);
    }
}
