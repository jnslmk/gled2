use std::num::NonZeroU64;

use crate::audio::state::{fft_data, fft_data_u8};
use crate::pipeline::constants::TEXTURE_SIZE;
use crate::pipeline::renderer_callback::RendererCallback;
use crate::storage::asset::scene::effect_state::OwnedTextureId;
use crate::ui::window_common::{default_viewport_builder, gled_window_frame};
use crate::wgpu_render_state;
use egui::load::SizedTexture;
use egui::{Color32, Context, Frame, Id, Image, Ui, ViewportId};
use egui_knob::{Knob, KnobStyle, LabelPosition};
use emath::{pos2, vec2, Pos2, Rect, Vec2};
use epaint::{PathStroke, Stroke, StrokeKind};
use wgpu::util::DeviceExt;
use wgpu::*;

pub struct ADSREditor {
    adsr: Adsr,
    low_pass: LowPass,
    open: bool,
    spectrum_pipeline: RenderPipeline,
    spectrum_bind_group: BindGroup,
    spectrum_texture_buffer: Buffer,
    spectrum_texture_view: TextureView,
    spectrum_texture_id: OwnedTextureId,
}

impl Default for ADSREditor {
    fn default() -> Self {
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
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            label: None,
            view_formats: &[TextureFormat::Bgra8Unorm],
        };

        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;

        let spectrum_texture = device.create_texture(&texture_desc);
        let spectrum_texture_view = spectrum_texture.create_view(&TextureViewDescriptor::default());

        let spectrum_vertex_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("audio spectrum vertex shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/vertex.wgsl").into()),
        });

        let spectrum_fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("audio spectrum fragment shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/audio_spectrum.wgsl").into()),
        });

        let spectrum_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("audio spectrum bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(1024),
                    },
                    count: None,
                }],
            });

        let spectrum_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("audio spectrum pipeline layout"),
            bind_group_layouts: &[&spectrum_bind_group_layout],
            push_constant_ranges: &[],
        });

        let spectrum_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            cache: None,
            label: Some("audio spectrum pipeline"),
            layout: Some(&spectrum_pipeline_layout),
            vertex: VertexState {
                module: &spectrum_vertex_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &spectrum_fragment_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(TextureFormat::Bgra8Unorm.into())],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let contents = [0u8; 1024];
        let spectrum_texture_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("audio spectrum uniform buffer"),
            contents: &contents,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let spectrum_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("audio spectrum bind group"),
            layout: &spectrum_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: spectrum_texture_buffer.as_entire_binding(),
            }],
        });

        let spectrum_texture_id =
            OwnedTextureId(wgpu_render_state.renderer.write().register_native_texture(
                &device,
                &spectrum_texture_view,
                wgpu::FilterMode::Nearest,
            ));

        Self {
            adsr: Adsr::new(AdsrParams::default(), 100.),
            low_pass: LowPass::new(400., 200., 100.),
            open: true,
            spectrum_pipeline,
            spectrum_bind_group,
            spectrum_texture_buffer,
            spectrum_texture_view,
            spectrum_texture_id,
        }
    }
}

impl ADSREditor {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        self.draw_spectrum_texture();

        ctx.show_viewport_immediate(
            ViewportId(Id::new("ADSR Editor")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(660.0, 500.0))
                .with_min_inner_size(Vec2::new(660.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "ADSR Editor", |ui| {
                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.draw_curve(ui);
                        let size = ui.available_size().min(Vec2::splat(300.0));
                        let spectrum =
                            Image::new(SizedTexture::new(self.spectrum_texture_id.0, size));
                        ui.add(spectrum);

                        ui.horizontal(|ui| {
                            Frame::new().inner_margin(5.).show(ui, |ui| {
                                self.draw_controls(ui);
                            });
                        });
                    });
                });
            },
        );
    }

    fn draw_spectrum_texture(&self) {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;
        if let Some(mut view) = wgpu_render_state.queue.write_buffer_with(
            &self.spectrum_texture_buffer,
            0,
            NonZeroU64::new(1024).expect("Contents length is zero"),
        ) {
            view.copy_from_slice(&fft_data_u8());
        }
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations for scene editor"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Renderer Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.spectrum_texture_view,
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
            });
            render_pass.set_pipeline(&self.spectrum_pipeline);
            render_pass.set_bind_group(0, &self.spectrum_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        RendererCallback::add(encoder.finish());
    }

    fn draw_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            .log2().clamp(0.0, 1.0)
            self.draw_meter(ui, amp);

            Frame::new().inner_margin(5.).show(ui, |ui| {
                ui.vertical(|ui| {
                    // Sensitivity knob
                    ui.add(
                        knob_default(Knob::new(
                            &mut self.low_pass.trigger_happiness,
                            1.0,
                            100.0,
                            KnobStyle::Wiper,
                        ))
                        .with_size(50.0)
                        .with_label("Sensitivity", LabelPosition::Bottom),
                    );

                    // Threshold knob
                    ui.add(
                        knob_default(Knob::new(
                            &mut self.adsr.params.gate_threshold,
                            0.0,
                            1.0,
                            KnobStyle::Wiper,
                        ))
                        .with_size(30.0)
                        .with_label("Threshold", LabelPosition::Bottom),
                    );
                });
            });
            ui.separator();
            // ADSR knobs
            Frame::new().inner_margin(5.).show(ui, |ui| {
                ui.add(
                    knob_default(Knob::new(
                        &mut self.adsr.params.attack_duration,
                        0.0,
                        2.0,
                        KnobStyle::Wiper,
                    ))
                    .with_size(50.0)
                    .with_label("Attack", LabelPosition::Bottom),
                );

                ui.add(
                    knob_default(Knob::new(
                        &mut self.adsr.params.decay_duration,
                        0.0,
                        10.0,
                        KnobStyle::Wiper,
                    ))
                    .with_size(50.0)
                    .with_label("Decay", LabelPosition::Bottom),
                );
                ui.add(
                    knob_default(Knob::new(
                        &mut self.adsr.params.sustain_level,
                        0.0,
                        1.0,
                        KnobStyle::Wiper,
                    ))
                    .with_size(50.0)
                    .with_label("Sustain", LabelPosition::Bottom),
                );
                ui.add(
                    knob_default(Knob::new(
                        &mut self.adsr.params.release_duration,
                        0.0,
                        10.0,
                        KnobStyle::Wiper,
                    ))
                    .with_size(50.0)
                    .with_label("Release", LabelPosition::Bottom),
                );
            });
            ui.separator();
            self.draw_meter(ui, control_amp);
        });
    }

    fn draw_meter(&mut self, ui: &mut Ui, amp: f32) {
        let meter_height = 200.0;
        let (_, rect) = ui.allocate_space(Vec2::new(40.0, meter_height));
        ui.painter().rect_stroke(
            rect,
            0.,
            Stroke::new(2., Color32::WHITE),
            StrokeKind::Outside,
        );

        let mut value_rect = rect.clone();
        value_rect.min.y += meter_height * (1.0 - amp);
        ui.painter()
            .rect_filled(value_rect, 0., Color32::LIGHT_GREEN);

        let threshold_line_y = rect.min.y + meter_height * (1.0 - self.adsr.params.gate_threshold);
        ui.painter().line(
            vec![
                pos2(rect.min.x, threshold_line_y),
                pos2(rect.max.x, threshold_line_y),
            ],
            Stroke::new(2., Color32::BLUE),
        );
    }
    fn draw_curve(&self, ui: &mut Ui) {
        let n = 300;

        Frame::new()
            .outer_margin(5.0)
            .fill(Color32::from_gray(60))
            .show(ui, |child_ui| {
                let desired_size = child_ui.available_width() * vec2(1.0, 0.35);

                let (_id, rect) = child_ui.allocate_space(desired_size);
                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0),
                    rect,
                );

                let mut adsr = Adsr::new(self.adsr.params.clone(), 100.);

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let t = i as f32 / (n as f32);
                        let input = if 0.2 < t
                            && t < (0.4
                                + self.adsr.params.decay_duration * 0.4
                                + self.adsr.params.attack_duration * 0.4)
                        {
                            1.0
                        } else {
                            0.0
                        };
                        let y = -0.8 * adsr.tick(input) + 1.;
                        to_screen * pos2(t as f32, y)
                    })
                    .collect();

                let thickness = 1.0;
                let shape = epaint::Shape::line(points, PathStroke::new(thickness, Color32::WHITE));

                child_ui.painter().add(shape);
            });
    }
}

fn knob_default(knob: Knob) -> Knob {
    knob.with_font_size(12.0)
        .with_colors(egui::Color32::GRAY, Color32::WHITE, Color32::WHITE)
        .with_stroke_width(3.0)
}
