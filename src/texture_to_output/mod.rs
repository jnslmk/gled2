//! Extract colors out of a texture into output buffers

mod positions;

use crate::{
    constants::{OUTPUT_BUFFER_SIZE, POSITIONS_BUFFER_SIZE, UNIVERSES},
    effect::EffectState,
    storage::Palette,
    wgpu_render_state,
};
use log::debug;
use std::num::{NonZero, NonZeroU64};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

pub use positions::{Lamp, Positions, Universe};

#[derive(Debug)]
pub struct TextureToOutput {
    pipeline: ComputePipeline,
    bind_group: BindGroup,
    positions: Buffer,
    output: Buffer,
}

impl TextureToOutput {
    pub fn init(texture: &Texture, uniforms: &Buffer) -> Self {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;

        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("TextureToArtnet shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/texture_to_output.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("TextureToArtnet bind group layout"),
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
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            { EffectState::size() + Palette::size() } as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("TextureToArtnet pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            cache: None,
            label: Some("TextureToArtnet pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let positions_contents: [u8; POSITIONS_BUFFER_SIZE as usize] = Positions::default().into();
        let positions = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("TextureToArtnet positions buffer"),
            contents: &positions_contents,
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("TextureToArtnet texture sampler"),
            ..Default::default()
        });

        let output = device.create_buffer(&BufferDescriptor {
            size: OUTPUT_BUFFER_SIZE,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            label: Some("TextureToArtnet output buffer gpu"),
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("TextureToArtnet bind group"),
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
                    resource: output.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: uniforms.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            bind_group,
            positions,
            output,
        }
    }

    pub fn set_positions(&self, queue: &Queue, positions: Positions) {
        debug!("Sending positions to gpu");
        let positions_contents: [u8; POSITIONS_BUFFER_SIZE as usize] = positions.into();

        if let Some(mut view) = queue.write_buffer_with(
            &self.positions,
            0,
            NonZero::new(positions_contents.len() as u64)
                .expect("positions_contents length is zero"),
        ) {
            view.copy_from_slice(&positions_contents);
        }
    }

    pub fn run(&self, encoder: &mut CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("TextureToArtnet compute pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(UNIVERSES as u32, 43, 1);
    }

    pub fn output_buffer(&self) -> &Buffer {
        &self.output
    }
}
