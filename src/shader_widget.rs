use eframe::{
    egui_wgpu::{self, wgpu},
    wgpu::util::DeviceExt,
};
use egui::{Rect, TextureId, Ui};
use std::{num::NonZeroU64, sync::Arc, time::Instant};

pub struct TriangleRenderResources {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub output_buffer: wgpu::Buffer,
}

impl TriangleRenderResources {
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

        queue.write_buffer(
            &self.uniform_buffer,
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
                .collect::<Vec<u8>>(),
        );
    }

    pub fn paint<'rp>(&'rp self, render_pass: &mut wgpu::RenderPass<'rp>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

pub fn custom_painting(start: Instant, ui: &mut egui::Ui, texture_id: &TextureId) {
    ui.image(*texture_id, egui::Vec2::splat(800.0));

    // The callback function for WGPU is in two stages: prepare, and paint.
    //
    // The prepare callback is called every frame before paint and is given access to the wgpu
    // Device and Queue, which can be used, for instance, to update buffers and uniforms before
    // rendering.
    //
    // You can use the main `CommandEncoder` that is passed-in, return an arbitrary number
    // of user-defined `CommandBuffer`s, or both.
    // The main command buffer, as well as all user-defined ones, will be submitted together
    // to the GPU in a single call.
    //
    // The paint callback is called after prepare and is given access to the render pass, which
    // can be used to issue draw commands.
    let cb = egui_wgpu::CallbackFn::new()
        .prepare(move |device, queue, _encoder, paint_callback_resources| {
            let resources: &TriangleRenderResources = paint_callback_resources.get().unwrap();
            resources.prepare(device, queue, start);

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GIF Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &resources.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
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

            resources.paint(&mut rpass);

            drop(rpass);

            /*
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &resources.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &resources.output_buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: padded_bytes_per_row,
                        rows_per_image: texture_size,
                    },
                },
                render_target.desc.size,
            );

            // Create the map request
            let buffer_slice = resources.output_buffer.slice(..);
            let request = buffer_slice.map_async(wgpu::MapMode::Read);
            // wait for the GPU to finish
            device.poll(wgpu::Maintain::Wait);
            let result = request.await;

            match result {
                Ok(()) => {
                    let padded_data = buffer_slice.get_mapped_range();
                    let data = padded_data
                        .chunks(padded_bytes_per_row as _)
                        .map(|chunk| &chunk[..unpadded_bytes_per_row as _])
                        .flatten()
                        .map(|x| *x)
                        .collect::<Vec<_>>();
                    drop(padded_data);
                    output_buffer.unmap();
                    frames.push(data);
                }
                _ => {
                    eprintln!("Something went wrong")
                }
            }
            */

            vec![encoder.finish()]
        })
        .paint(move |_info, render_pass, paint_callback_resources| {
            let resources: &TriangleRenderResources = paint_callback_resources.get().unwrap();
            //resources.paint(render_pass);
        });

    let callback = egui::PaintCallback {
        rect: Rect::NOTHING,
        callback: Arc::new(cb),
    };

    ui.painter().add(callback);
}

pub fn init_shader<'a>(cc: &'a eframe::CreationContext<'a>) -> TextureId {
    // Get the WGPU render state from the eframe creation context. This can also be retrieved
    // from `eframe::Frame` when you don't have a `CreationContext` available.
    let wgpu_render_state = cc
        .wgpu_render_state
        .as_ref()
        .expect("Could not get wgpu render state");

    let device = &wgpu_render_state.device;

    let texture_size = 1024u32;
    let rt_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: texture_size,
            height: texture_size,
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
    };

    let texture = device.create_texture(&rt_desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // wgpu requires texture -> buffer copies to be aligned using
    // wgpu::COPY_BYTES_PER_ROW_ALIGNMENT. Because of this we'll
    // need to save both the padded_bytes_per_row as well as the
    // unpadded_bytes_per_row
    let pixel_size = std::mem::size_of::<[u8; 4]>() as u32;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let unpadded_bytes_per_row = pixel_size * texture_size;
    let padding = (align - unpadded_bytes_per_row % align) % align;
    let padded_bytes_per_row = unpadded_bytes_per_row + padding;

    // create a buffer to copy the texture to so we can get the data
    let buffer_size = (padded_bytes_per_row * texture_size) as wgpu::BufferAddress;
    let buffer_desc = wgpu::BufferDescriptor {
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: Some("Output Buffer"),
        mapped_at_creation: false,
    };
    let output_buffer = device.create_buffer(&buffer_desc);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("custom3d"),
        source: wgpu::ShaderSource::Wgsl(include_str!("./custom3d_wgpu_shader.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("custom3d"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(272),
            },
            count: None,
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("custom3d"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("custom3d"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu_render_state.target_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("custom3d"),
        contents: bytemuck::cast_slice(&[0.0_f32; 68]), // 16 bytes aligned!
        // Mapping at creation (as done by the create_buffer_init utility) doesn't require us to to add the MAP_WRITE usage
        // (this *happens* to workaround this bug )
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("custom3d"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let texture_id = wgpu_render_state.renderer.write().register_native_texture(
        device,
        &view,
        wgpu::FilterMode::Nearest,
    );

    // Because the graphics pipeline must have the same lifetime as the egui render pass,
    // instead of storing the pipeline in our `Custom3D` struct, we insert it into the
    // `paint_callback_resources` type map, which is stored alongside the render pass.
    wgpu_render_state
        .renderer
        .write()
        .paint_callback_resources
        .insert(TriangleRenderResources {
            pipeline,
            bind_group,
            uniform_buffer,
            texture,
            view,
            output_buffer,
        });

    texture_id
}

pub fn render_shader_widget(start: Instant, ui: &mut Ui, texture_id: &TextureId) {
    egui::Frame::canvas(ui.style()).show(ui, move |ui| {
        custom_painting(start, ui, texture_id);
    });
}
