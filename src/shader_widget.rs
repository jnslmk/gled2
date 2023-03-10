use eframe::{egui_wgpu::wgpu, wgpu::util::DeviceExt};
use egui::TextureId;
use std::{
    num::{NonZeroU32, NonZeroU64},
    time::Instant,
};

// wgpu requires texture -> buffer copies to be aligned using
// wgpu::COPY_BYTES_PER_ROW_ALIGNMENT. Because of this we'll
// need to save both the padded_bytes_per_row as well as the
// unpadded_bytes_per_row
const TEXTURE_SIZE: u32 = 1024u32;
const PIXEL_SIZE: u32 = std::mem::size_of::<[u8; 4]>() as u32;
const ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
const UNPADDED_BYTES_PER_ROW: u32 = PIXEL_SIZE * TEXTURE_SIZE;
const PADDING: u32 = (ALIGN - UNPADDED_BYTES_PER_ROW % ALIGN) % ALIGN;
const PADDED_BYTES_PER_ROW: u32 = UNPADDED_BYTES_PER_ROW + PADDING;

pub struct TriangleRenderResources {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub input_buffer: wgpu::Buffer,
    pub _work_buffer: wgpu::Buffer,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub output_buffer: wgpu::Buffer,
    pub texture_desc: wgpu::TextureDescriptor<'static>,
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
}

pub fn init_shader<'a>(cc: &'a eframe::CreationContext<'a>) -> TextureId {
    // Get the WGPU render state from the eframe creation context. This can also be retrieved
    // from `eframe::Frame` when you don't have a `CreationContext` available.
    let wgpu_render_state = cc
        .wgpu_render_state
        .as_ref()
        .expect("Could not get wgpu render state");

    let device = &wgpu_render_state.device;

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
    };

    let texture = device.create_texture(&texture_desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // create a buffer to copy the texture to so we can get the data
    let output_buffer_size = (PADDED_BYTES_PER_ROW * TEXTURE_SIZE) as wgpu::BufferAddress;
    let output_buffer_desc = wgpu::BufferDescriptor {
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: Some("Output Buffer"),
        mapped_at_creation: false,
    };
    let output_buffer = device.create_buffer(&output_buffer_desc);

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
            targets: &[Some(wgpu_render_state.target_format.into())],
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

    let work_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("work_buffer"),
        contents: &[0u8; 16_384],
        usage: wgpu::BufferUsages::STORAGE,
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
        ],
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
            input_buffer,
            texture,
            view,
            output_buffer,
            _work_buffer: work_buffer,
            texture_desc,
        });

    texture_id
}

pub fn render(frame: &eframe::Frame, start: Instant) {
    let wgpu_render_state = frame
        .wgpu_render_state()
        .expect("Could not get wgpu render state");

    let device = &wgpu_render_state.device;
    let queue = &wgpu_render_state.queue;

    let renderer = wgpu_render_state.renderer.read();
    let resources: &TriangleRenderResources = renderer.paint_callback_resources.get().unwrap();

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
                bytes_per_row: NonZeroU32::new(PADDED_BYTES_PER_ROW),
                rows_per_image: NonZeroU32::new(TEXTURE_SIZE),
            },
        },
        resources.texture_desc.size,
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Create the map request
    let buffer_slice = resources.output_buffer.slice(..);
    let (tx, rx) = crossbeam_channel::bounded(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });

    device.poll(wgpu::Maintain::Wait);

    match rx.recv().unwrap() {
        Ok(()) => {
            let padded_data = buffer_slice.get_mapped_range();
            /*let data = padded_data
            .chunks(padded_bytes_per_row as _)
            .flat_map(|chunk| &chunk[..unpadded_bytes_per_row as _])
            .copied()
            .collect::<Vec<_>>();
            */
            //dbg!(&padded_data[..4]);
            drop(padded_data);

            resources.output_buffer.unmap();
        }
        _ => {
            eprintln!("Something went wrong")
        }
    }
}
