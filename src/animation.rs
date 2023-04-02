//! Renders to a texture

use std::{num::NonZeroU64, time::Instant};
use wgpu::{util::DeviceExt, *};

// wgpu requires texture -> buffer copies to be aligned using
// COPY_BYTES_PER_ROW_ALIGNMENT. Because of this we'll
// need to save both the padded_bytes_per_row as well as the
// unpadded_bytes_per_row
const TEXTURE_SIZE: u32 = 1024u32;

pub struct Animation {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    input_buffer: Buffer,
    _work_buffer: Buffer,
    texture: Texture,
    view: TextureView,
}

impl Animation {
    pub fn prepare(&self, _device: &Device, queue: &Queue, start: Instant) {
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

    pub fn init(device: &Device, target_format: TextureFormat) -> Self {
        let texture_desc = TextureDescriptor {
            size: Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8Unorm,
            usage: TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING,
            label: None,
            view_formats: &[TextureFormat::Bgra8Unorm],
        };

        let texture = device.create_texture(&texture_desc);
        let view = texture.create_view(&TextureViewDescriptor::default());

        let vertex_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("vertex shader"),
            source: ShaderSource::Wgsl(include_str!("./vertex_shader.wgsl").into()),
        });

        let fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("fragment shader"),
            source: ShaderSource::Wgsl(include_str!("./fragment_shader.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(19 * 16),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(4096),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &fragment_shader,
                entry_point: "fs_main",
                targets: &[Some(target_format.into())],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let input_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("input_buffer"),
            contents: &[0u8; 19 * 16], // 16 bytes aligned!
            // Mapping at creation (as done by the create_buffer_init utility) doesn't require us to to add the MAP_WRITE usage
            // (this *happens* to workaround this bug )
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let work_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("work_buffer"),
            contents: &[0u8; 16_384],
            usage: BufferUsages::STORAGE,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("bind_group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: work_buffer.as_entire_binding(),
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
        }
    }

    pub fn render(&self, device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("GIF Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: self.view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn view(&self) -> &TextureView {
        &self.view
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}
