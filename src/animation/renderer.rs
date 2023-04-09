//! Renders to a texture
use super::{config::Config, state::State, ColorPalette};
use std::num::NonZeroU64;
use wgpu::{util::DeviceExt, *};

pub const TEXTURE_SIZE: u32 = 2048u32;

static COMMON_SHADER_CODE: &str = include_str!("../shaders/common.wgsl");

pub struct AnimationRenderer {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    uniform: Buffer,
    texture: Texture,
    view: TextureView,
}

impl AnimationRenderer {
    pub fn new(device: &Device, animation_shader: &str, config: &Config) -> Self {
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
            label: Some("animation vertex shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/vertex.wgsl").into()),
        });

        let mut fragment_shader = COMMON_SHADER_CODE.to_owned();
        fragment_shader.push_str(animation_shader);

        let fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("animation fragment shader"),
            source: ShaderSource::Wgsl(fragment_shader.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("animation bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        (State::size() + ColorPalette::size() + Config::size()) as u64,
                    ),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("animation pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("animation pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &fragment_shader,
                entry_point: "fs_main",
                targets: &[Some(TextureFormat::Bgra8Unorm.into())],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let mut contents = [0u8; State::size() + ColorPalette::size() + Config::size()];
        config.write_data(
            &mut contents[State::size() + ColorPalette::size()
                ..State::size() + ColorPalette::size() + Config::size()],
        );

        let uniform = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("animation uniform buffer"),
            contents: &contents,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("animation bind group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            bind_group,
            uniform,
            texture,
            view,
        }
    }

    pub fn set_buffers(&self, queue: &Queue, state: &State, palette: &ColorPalette) {
        let mut contents = [0; State::size() + ColorPalette::size()];
        state.write_data(&mut contents[..State::size()]);
        palette.write_data(&mut contents[State::size()..State::size() + ColorPalette::size()]);

        queue.write_buffer(&self.uniform, 0, &contents);
    }

    pub fn render(&self, encoder: &mut CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("GIF Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: self.view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color {
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

    pub fn view(&self) -> &TextureView {
        &self.view
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}
