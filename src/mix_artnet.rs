//! Combine two artnet buffers into one artnet buffer.
use std::num::NonZeroU64;
use wgpu::*;

use crate::constants::{ARTNET_BUFFER_SIZE, LAMPS};

#[derive(Debug)]
pub struct MixArtnet {
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
}

impl MixArtnet {
    pub fn init(device: &Device) -> Self {
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("MixArtnet shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/mix_artnet.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("MixArtnet bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(ARTNET_BUFFER_SIZE),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(ARTNET_BUFFER_SIZE),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("MixArtnet pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("MixArtnet pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "main",
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn run(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        main: &Buffer,
        other: &Buffer,
    ) {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("MixArtnet bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: main.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: other.as_entire_binding(),
                },
            ],
        });

        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("MixArtnet compute pass"),
        });
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(LAMPS as u32, 1, 1);
    }
}
