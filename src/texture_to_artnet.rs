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
pub const ARTNET_BUFFER_SIZE: u64 = UNIVERSES * 512;

pub struct TextureToArtnet {
    pipeline: ComputePipeline,
    bind_group: BindGroup,
    _positions: Buffer,
    artnet: Buffer,
}

impl TextureToArtnet {
    pub fn init(device: &Device, texture: &Texture, positions: &Positions) -> Self {
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("TextureToArtnet shader"),
            source: ShaderSource::Wgsl(
                include_str!("./texture_to_artnet/texture_to_artnet.wgsl").into(),
            ),
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
                        min_binding_size: NonZeroU64::new(ARTNET_BUFFER_SIZE),
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
            label: Some("TextureToArtnet pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "main",
        });

        let positions_contents: [u8; POSITIONS_BUFFER_SIZE as usize] = positions.into();
        let positions = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("TextureToArtnet positions buffer"),
            contents: &positions_contents,
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("TextureToArtnet texture sampler"),
            ..Default::default()
        });

        let artnet = device.create_buffer(&BufferDescriptor {
            size: ARTNET_BUFFER_SIZE,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
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
                    resource: artnet.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            bind_group,
            _positions: positions,
            artnet,
        }
    }

    pub fn run(&self, encoder: &mut CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("TextureToArtnet compute pass"),
        });
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(UNIVERSES as u32, 128, 1);
    }

    pub fn artnet_buffer(&self) -> &Buffer {
        &self.artnet
    }
}
