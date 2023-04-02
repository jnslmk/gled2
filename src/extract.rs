mod positions;

/// Extract colors out of a texture into artnet buffers
use eframe::{egui_wgpu::wgpu, wgpu::util::DeviceExt};
use egui::TextureId;
use std::{num::NonZeroU64, time::Instant};

use self::positions::Positions;

// wgpu requires texture -> buffer copies to be aligned using
// wgpu::COPY_BYTES_PER_ROW_ALIGNMENT. Because of this we'll
// need to save both the padded_bytes_per_row as well as the
// unpadded_bytes_per_row
const TEXTURE_SIZE: u32 = 1024u32;

pub struct Extract {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub positions: wgpu::Buffer,
    pub output: wgpu::Buffer,
    pub texture_desc: wgpu::TextureDescriptor<'static>,
}

impl Extract {
    pub fn set_positions(&self, queue: &wgpu::Queue, positions: &Positions) {
        positions.write_to_buffer(queue, &self.positions);
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
            pointer_buffer,
            start_address_buffer,
            texture,
            view,
            artnet_buffer_gpu,
            artnet_buffer_cpu,
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

    encoder.copy_buffer_to_buffer(
        &resources.artnet_buffer_gpu,
        0,
        &resources.artnet_buffer_cpu,
        0,
        65536,
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Create the map request
    let buffer_slice = resources.artnet_buffer_cpu.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
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

            resources.artnet_buffer_cpu.unmap();
        }
        _ => {
            eprintln!("Something went wrong")
        }
    }
}
