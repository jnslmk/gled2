//! Renders to a texture
use crate::{
    audio::sound_data::SoundData,
    pipeline::constants::TEXTURE_SIZE,
    storage::{Palette, collections::Collections, scene::effect_state::EffectState},
    wgpu_render_state,
};
use std::num::{NonZero, NonZeroU64};
use wgpu::{util::DeviceExt, *};

#[derive(Debug, PartialEq)]
pub struct AnimationRenderer {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    uniform: Buffer,
    texture: Texture,
    view: TextureView,
}

impl AnimationRenderer {
    pub fn new(fragment_shader: &str) -> Self {
        let texture_desc = TextureDescriptor {
            size: Extent3d {
                width: TEXTURE_SIZE as u32,
                height: TEXTURE_SIZE as u32,
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

        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;

        let texture = device.create_texture(&texture_desc);
        let view = texture.create_view(&TextureViewDescriptor::default());

        let vertex_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("animation vertex shader"),
            source: ShaderSource::Wgsl(include_str!("../../../shaders/vertex.wgsl").into()),
        });

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
                        { EffectState::size() + Palette::size() } as u64
                    ),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("animation pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            cache: None,
            label: Some("animation pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vertex_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &fragment_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(TextureFormat::Bgra8Unorm.into())],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
        });

        let contents = [0u8; EffectState::size() + Palette::size()];
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

    pub fn set_buffers(
        &self,
        queue: &Queue,
        state: &EffectState,
        beat_progression: f32,
        palette: Option<Palette>,
        collections: &Collections,
        sound_data: &SoundData,
    ) {
        let mut contents = [0; Palette::size() + EffectState::size()];
        if let Some(palette) = palette {
            palette.write_data(&mut contents[..Palette::size()]);
        }
        state.write_data(
            beat_progression,
            &mut contents[Palette::size()..Palette::size() + EffectState::size()],
            collections,
            sound_data,
        );

        if let Some(mut view) = queue.write_buffer_with(
            &self.uniform,
            0,
            NonZero::new(contents.len() as u64).expect("Contents length is zero"),
        ) {
            view.copy_from_slice(&contents);
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn render(&self, encoder: &mut CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Renderer Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: self.view(),
                resolve_target: None,
                depth_slice: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
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

    pub fn uniforms(&self) -> &Buffer {
        &self.uniform
    }
}
