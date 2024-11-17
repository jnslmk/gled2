//! Calculate preview indices from preview positions.
//! Each pixel position gets a u16 which is:
//!  * 0xffffffff if it should stay black.
//!  * index of output buffer where the color triplet starts.
use crate::{
    app::preview_positions,
    constants::{
        LAMPS_PER_UNIVERSE, POSITIONS_BUFFER_SIZE, PREVIEW_INDICES_BUFFER_SIZE,
        PREVIEW_TEXTURE_SIZE, UNIVERSES,
    },
    wgpu_render_state,
};
use egui::mutex::Mutex;
use log::debug;
use once_cell::sync::Lazy;
use std::num::NonZeroU64;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

pub static PREVIEW_INDICES: Lazy<Mutex<PreviewIndices>> =
    Lazy::new(|| Mutex::new(PreviewIndices::new()));

#[derive(Debug)]
pub struct PreviewIndices {
    positions: Buffer,
    indices: Buffer,
    clear_bind_group: BindGroup,
    clear_pipeline: ComputePipeline,
    index_bind_group: BindGroup,
    index_pipeline: ComputePipeline,
    pub send_positions: bool,
}

impl PreviewIndices {
    pub fn new() -> Self {
        let device = wgpu_render_state().device;

        let positions = preview_positions();
        let positions_contents: [u8; POSITIONS_BUFFER_SIZE as usize] = positions.into();
        let positions = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Preview positions buffer"),
            contents: &positions_contents,
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
        });

        let indices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Preview indices buffer"),
            contents: &[0; PREVIEW_INDICES_BUFFER_SIZE as usize],
            usage: BufferUsages::STORAGE,
        });

        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;

        let clear_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("preview indices clear bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(PREVIEW_INDICES_BUFFER_SIZE),
                },
                count: None,
            }],
        });

        let clear_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("animation pipeline layout"),
            bind_group_layouts: &[&clear_bind_group_layout],
            push_constant_ranges: &[],
        });

        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("preview clear indices shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/preview_indices_clear.wgsl").into()),
        });

        let clear_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            cache: None,
            label: Some("preview clear indices pipeline"),
            layout: Some(&clear_pipeline_layout),
            module: &module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let clear_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("preview indices clear bind group"),
            layout: &clear_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: indices.as_entire_binding(),
            }],
        });

        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("preview index indices shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/preview_indices_index.wgsl").into()),
        });

        let index_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("preview indices index bind group layout"),
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
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(PREVIEW_INDICES_BUFFER_SIZE),
                    },
                    count: None,
                },
            ],
        });

        let index_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("animation pipeline layout"),
            bind_group_layouts: &[&index_bind_group_layout],
            push_constant_ranges: &[],
        });

        let index_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            cache: None,
            label: Some("preview index indices pipeline"),
            layout: Some(&index_pipeline_layout),
            module: &module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let index_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("preview indices index bind group"),
            layout: &index_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: positions.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: indices.as_entire_binding(),
                },
            ],
        });

        Self {
            positions,
            indices,
            send_positions: false,
            clear_bind_group,
            clear_pipeline,
            index_bind_group,
            index_pipeline,
        }
    }

    pub fn prepare(&mut self, queue: &Queue) {
        if !self.send_positions {
            return;
        }

        debug!("Sending positions to gpu");
        let positions = preview_positions();
        let positions_contents: [u8; POSITIONS_BUFFER_SIZE as usize] = positions.into();
        queue.write_buffer(&self.positions, 0, &positions_contents);
    }

    pub fn run(&mut self, encoder: &mut CommandEncoder) {
        if !self.send_positions {
            return;
        }
        self.send_positions = false;

        debug!("Calculating indices on the gpu");
        {
            let mut clear_compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Preview indices clear compute pass"),
                timestamp_writes: None,
            });
            clear_compute_pass.set_pipeline(&self.clear_pipeline);
            clear_compute_pass.set_bind_group(0, &self.clear_bind_group, &[]);
            clear_compute_pass.dispatch_workgroups(
                PREVIEW_TEXTURE_SIZE as u32 / 2,
                PREVIEW_TEXTURE_SIZE as u32,
                1,
            );
        }

        let mut index_compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Preview indices index compute pass"),
            timestamp_writes: None,
        });
        index_compute_pass.set_pipeline(&self.index_pipeline);
        index_compute_pass.set_bind_group(0, &self.index_bind_group, &[]);
        index_compute_pass.dispatch_workgroups(UNIVERSES as u32, LAMPS_PER_UNIVERSE as u32, 1);
    }

    pub fn send_positions(&mut self) {
        self.send_positions = true;
    }

    pub fn indices(&self) -> &Buffer {
        &self.indices
    }
}
