//! Render preview circles.
use crate::{
    constants::{OUTPUT_BUFFER_SIZE, PREVIEW_INDICES_BUFFER_SIZE, PREVIEW_TEXTURE_SIZE},
    preview_indices::PreviewIndices,
    wgpu_render_state, OUTPUT_BUFFER,
};
use egui::TextureId;
use once_cell::unsync::OnceCell;
use std::{cell::RefCell, num::NonZeroU64};
use wgpu::*;

thread_local! {
    static PREVIEW: OnceCell<Preview> = const { OnceCell::new() };
}

#[derive(Debug)]
pub struct Preview {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    bind_group: RefCell<Option<BindGroup>>,
    view: TextureView,
    texture_id: TextureId,
}

impl Preview {
    fn init() -> Self {
        let texture_desc = TextureDescriptor {
            size: Extent3d {
                width: PREVIEW_TEXTURE_SIZE as u32,
                height: PREVIEW_TEXTURE_SIZE as u32,
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
            label: Some("preview vertex shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/vertex.wgsl").into()),
        });

        let fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("preview fragment shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/preview.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("preview bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(PREVIEW_INDICES_BUFFER_SIZE),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
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
            label: Some("preview pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            cache: None,
            label: Some("preview pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vertex_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &fragment_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(TextureFormat::Bgra8Unorm.into())],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let texture_id = wgpu_render_state.renderer.write().register_native_texture(
            &device,
            &view,
            wgpu::FilterMode::Nearest,
        );

        Self {
            pipeline,
            bind_group_layout,
            bind_group: RefCell::new(None),
            view,
            texture_id,
        }
    }

    pub fn set_buffers() {
        PREVIEW.with(|preview| {
            let preview = preview.get_or_init(Self::init);
            let device = wgpu_render_state().device;
            preview
                .bind_group
                .replace(Some(device.create_bind_group(&BindGroupDescriptor {
                    label: Some("Preview bind group"),
                    layout: &preview.bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: PreviewIndices::get().indices().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: OUTPUT_BUFFER.as_entire_binding(),
                        },
                    ],
                })));
        });
    }

    pub fn run(encoder: &mut CommandEncoder) {
        PREVIEW.with(|preview| {
            let preview = preview.get_or_init(Self::init);

            if let Some(bind_group) = &*preview.bind_group.borrow() {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Renderer Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &preview.view,
                        resolve_target: None,
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
                });
                render_pass.set_pipeline(&preview.pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
        });
    }

    pub fn texture_id() -> TextureId {
        PREVIEW.with(|preview| preview.get_or_init(Self::init).texture_id)
    }
}
