/// Renders to a texture
use std::{num::NonZeroU64, time::Instant};
use wgpu::{util::DeviceExt, Device, TextureFormat};

// wgpu requires texture -> buffer copies to be aligned using
// wgpu::COPY_BYTES_PER_ROW_ALIGNMENT. Because of this we'll
// need to save both the padded_bytes_per_row as well as the
// unpadded_bytes_per_row
const TEXTURE_SIZE: u32 = 1024u32;

pub struct Animation {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub input_buffer: wgpu::Buffer,
    pub _work_buffer: wgpu::Buffer,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub texture_desc: wgpu::TextureDescriptor<'static>,
}

impl Animation {
    pub fn prepare(&self, _device: &wgpu::Device, queue: &wgpu::Queue, start: Instant) {
        let time = start.elapsed().as_secs_f32() % 1.0;
        let colors: Vec<(f32, f32, f32)> = vec![
            (1., 0., 0.7),
            (0., 0.2, 0.2),
            (0., 0., 0.),
            (0., 0.4, 0.5),
            (0., 1., 0.),
        ];
        let colors_count = colors.len();

        let center = &[0.75f32, 0.25];
        let thickness: f32 = 0.01;
        let count: i32 = 12;
        let frame_rate: f32 = 91.0;

        queue.write_buffer(
            &self.input_buffer,
            0,
            &time
                .to_le_bytes()
                .into_iter()
                .chain((colors_count as i32).to_le_bytes().into_iter())
                .chain(std::iter::repeat(0u8).take(8))
                .chain(colors.into_iter().flat_map(|(r, g, b)| {
                    r.to_le_bytes()
                        .into_iter()
                        .chain(g.to_le_bytes().into_iter())
                        .chain(b.to_le_bytes().into_iter())
                        .chain(std::iter::repeat(0u8).take(4))
                }))
                .chain(std::iter::repeat(0u8).take((16 - colors_count) * 16))
                .chain(center[0].to_le_bytes().into_iter())
                .chain(center[1].to_le_bytes().into_iter())
                .chain(thickness.to_le_bytes().into_iter())
                .chain(count.to_le_bytes().into_iter())
                .chain(frame_rate.to_le_bytes().into_iter())
                .chain(std::iter::repeat(0u8).take(12))
                .collect::<Vec<u8>>(),
        );
    }

    pub fn paint<'rp>(&'rp self, render_pass: &mut wgpu::RenderPass<'rp>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    pub fn init(device: &Device, target_format: TextureFormat) -> Self {
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
            view_formats: &[wgpu::TextureFormat::Bgra8Unorm],
        };

        let texture = device.create_texture(&texture_desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vertex shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./vertex_shader.wgsl").into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fragment shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./fragment_shader.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(19 * 16),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(4096),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(1024 * 1024 * 2),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(65536 * 2),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(65536),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: "fs_main",
                targets: &[Some(target_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("input_buffer"),
            contents: &[0u8; 19 * 16], // 16 bytes aligned!
            // Mapping at creation (as done by the create_buffer_init utility) doesn't require us to to add the MAP_WRITE usage
            // (this *happens* to workaround this bug )
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let pointer_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pointer_buffer"),
            contents: &[0u8; 1024 * 1024 * 2], // 16 bytes aligned!
            // Mapping at creation (as done by the create_buffer_init utility) doesn't require us to to add the MAP_WRITE usage
            // (this *happens* to workaround this bug )
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        let start_address_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("start_address_buffer"),
            contents: &[0u8; 131_072], // 16 bytes aligned!
            // Mapping at creation (as done by the create_buffer_init utility) doesn't require us to to add the MAP_WRITE usage
            // (this *happens* to workaround this bug )
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        let work_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("work_buffer"),
            contents: &[0u8; 16_384],
            usage: wgpu::BufferUsages::STORAGE,
        });

        let artnet_buffer_gpu = device.create_buffer(&wgpu::BufferDescriptor {
            size: 65536,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            label: Some("Artnet Buffer"),
            mapped_at_creation: false,
        });
        let artnet_buffer_cpu = device.create_buffer(&wgpu::BufferDescriptor {
            size: 65536,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            label: Some("Artnet Buffer"),
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: work_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: pointer_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: start_address_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: artnet_buffer_gpu.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            bind_group,
            input_buffer,
            texture,
            view,
            _work_buffer: work_buffer,
            texture_desc,
        }
    }
}
