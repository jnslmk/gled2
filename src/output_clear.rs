//! Clear output buffer.
use crate::{constants::OUTPUT_BUFFER_SIZE, wgpu_render_state};
use std::num::NonZeroU64;
use wgpu::*;

#[derive(Debug)]
pub struct OutputClear {
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
    bind_group: Option<BindGroup>,
}

impl OutputClear {
    pub fn init() -> Self {
        let device = wgpu_render_state().device;

        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("OutputClear shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/output_clear.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("OutputClear bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(OUTPUT_BUFFER_SIZE),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("OutputClear pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            cache: None, //TODO: Cache
            label: Some("OutputClear pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        Self {
            pipeline,
            bind_group_layout,
            bind_group: None,
        }
    }

    pub fn set_buffers(&mut self, output: &Buffer) {
        let device = wgpu_render_state().device;
        self.bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: Some("OutputClear bind group"),
            layout: &self.bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: output.as_entire_binding(),
            }],
        }));
    }
    pub fn run(&self, encoder: &mut CommandEncoder) {
        if let Some(bind_group) = self.bind_group.as_ref() {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("OutputClear compute pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            compute_pass.dispatch_workgroups(OUTPUT_BUFFER_SIZE as u32 / 4, 1, 1);
        }
    }
}
