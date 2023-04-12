//! Combine two artnet buffers into one artnet buffer.
use crate::{constants::ARTNET_BUFFER_SIZE, wgpu_render_state};
use std::num::NonZeroU64;
use wgpu::*;

#[derive(Debug)]
pub struct ArtnetClear {
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
    bind_group: Option<BindGroup>,
}

impl ArtnetClear {
    pub fn init() -> Self {
        let device = wgpu_render_state().device;

        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("ArtnetClear shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/artnet_clear.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ArtnetClear bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(ARTNET_BUFFER_SIZE),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("ArtnetClear pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("ArtnetClear pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "main",
        });

        Self {
            pipeline,
            bind_group_layout,
            bind_group: None,
        }
    }

    pub fn set_buffers(&mut self, artnet: &Buffer) {
        let device = wgpu_render_state().device;
        self.bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: Some("ArtnetClear bind group"),
            layout: &self.bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: artnet.as_entire_binding(),
            }],
        }));
    }
    pub fn run(&self, encoder: &mut CommandEncoder) {
        if let Some(bind_group) = self.bind_group.as_ref() {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("ArtnetClear compute pass"),
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            compute_pass.dispatch_workgroups(ARTNET_BUFFER_SIZE as u32 / 4, 1, 1);
        }
    }
}
