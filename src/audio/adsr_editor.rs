use crate::audio::reactive_signal::{AdsrParams, ReactiveSignal};
use crate::audio::state::{fft_data_u8, MAX_FREQ};
use crate::audio::{ReactiveSignalHandle, REACTIVE_SIGNAL_THREAD};
use crate::pipeline::constants::TEXTURE_SIZE;
use crate::pipeline::renderer_callback::RendererCallback;
use crate::storage::asset::scene::effect_state::OwnedTextureId;
use crate::ui::scoped_frame;
use crate::wgpu_render_state;
use egui::load::SizedTexture;
use egui::{Color32, Frame, Image, Layout, Ui, UiBuilder};
use egui_knob::{Knob, KnobStyle, LabelPosition};
use emath::{pos2, remap_clamp, vec2, Align, Pos2, Rect, Vec2};
use epaint::{PathShape, PathStroke, Stroke};
use ndarray::Array1;
use std::num::NonZeroU64;
use std::sync::MutexGuard;
use wgpu::util::DeviceExt;
use wgpu::*;

#[derive(PartialEq, Clone, Debug)]
pub struct ADSREditor {
    pub(crate) reactive_signal_handle: ReactiveSignalHandle,
    f_center: f32,
    f_radius: f32,
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

        let reactive_signal_handle = REACTIVE_SIGNAL_THREAD.write().unwrap().register_reactive_signal();

        Self {
            reactive_signal_handle,
            f_center: 8200.,
            f_radius: 990.,
            spectrum_pipeline,
            spectrum_bind_group,
            spectrum_texture_buffer,
            spectrum_texture_view,
            spectrum_texture_id,
        }
    }
}

impl ADSREditor {
    pub fn show(&mut self, ui: &mut Ui) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!("ADSREditor::show");
        Frame::new().inner_margin(5.).show(ui, |ui| {
            let signal = self.reactive_signal_handle.update_params_and_fetch_signal().unwrap_or_default();
            let spectrum = signal.spectrum.clone();
            let impulse = signal.impulse.clamp(0.0, 1.0);
            let output_level = signal.current_level;

            self.draw_spectrum_texture(spectrum);

            self.draw_spectrum(ui);
            ui.separator();
            self.draw_adsr(ui, impulse, output_level);
        });
    }

    pub fn show_minified(&mut self, ui: &mut Ui, rect: Rect) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!("ADSREditor::show_minified");
        let level = self.reactive_signal_handle.level();
        scoped_frame(ui, UiBuilder::new().max_rect(rect), Frame::default(), |ui| {
            let to_screen = emath::RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO, Vec2::new(4.0, 1.0)),
                rect,
            );
            let painter = ui.painter().with_clip_rect(rect);

            for i in 0..=4 {
                let stroke = Stroke::new(
                    match i {
                        0 | 4 => 1.0,
                        _ => 0.5,
                    },
                    Color32::GRAY,
                );
                painter.add(PathShape::line(
                    vec![
                        to_screen.transform_pos(Pos2::new(i as f32, 0.0)),
                        to_screen.transform_pos(Pos2::new(i as f32, 1.0)),
                    ],
                    stroke,
                ));
                painter.add(PathShape::line(
                    vec![
                        to_screen.transform_pos(Pos2::new(0.0, i as f32 * 0.25)),
                        to_screen.transform_pos(Pos2::new(4.0, i as f32 * 0.25)),
                    ],
                    stroke,
                ));
            }

            self.draw_curve(ui, rect, level);
        });
    }

    fn lock_params(&self) -> MutexGuard<'_, AdsrParams> {
        self.reactive_signal_handle.params.lock().unwrap()
    }

    fn draw_spectrum_texture(&self, input_level: Array1<f32>) {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;
        if let Some(mut view) = wgpu_render_state.queue.write_buffer_with(
            &self.spectrum_texture_buffer,
            0,
            NonZeroU64::new(1024).expect("Contents length is zero"),
        ) {
            view.copy_from_slice(&fft_data_u8(input_level.to_vec()));
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

    fn draw_adsr(&mut self, ui: &mut Ui, input_level: f32, output_level: f32) {
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            let meter_height = 200.0;
            let (_, rect) = ui.allocate_space(Vec2::new(40.0, meter_height));
            ui.painter().rect_filled(rect, 0., Color32::from_gray(100));

            let mut value_rect = rect.clone();
            value_rect.min.y += meter_height * (1.0 - input_level);
            ui.painter()
                .rect_filled(value_rect, 0., Color32::LIGHT_GREEN);

            let threshold_line_y =
                rect.min.y + meter_height * (1.0 - self.reactive_signal_handle.params.lock().unwrap().gate_activation_threshold);
            ui.painter().line(
                vec![
                    pos2(rect.min.x, threshold_line_y),
                    pos2(rect.max.x, threshold_line_y),
                ],
                Stroke::new(2., Color32::BLUE),
            );

            let threshold_deac_line_y =
                rect.min.y + meter_height * (1.0 - self.reactive_signal_handle.params.lock().unwrap().gate_deactivation_threshold);
            ui.painter().line(
                vec![
                    pos2(rect.min.x, threshold_deac_line_y),
                    pos2(rect.max.x, threshold_deac_line_y),
                ],
                Stroke::new(2., Color32::BLUE),
            );

            let mut rect = ui.available_rect_before_wrap();
            rect.set_height(200.);
            let frame = Frame::new().inner_margin(5.0).fill(Color32::from_gray(60));
            scoped_frame(ui, UiBuilder::new().max_rect(rect), frame, |ui| {
                let rect = ui.available_rect_before_wrap();
                self.draw_curve(ui, rect, output_level);
            });
        });
        scoped_frame(
            ui,
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    ui.cursor().min,
                    vec2(ui.available_width(), 100.),
                ))
                .layout(Layout::left_to_right(Align::Max).with_cross_justify(true)),
            Frame::default().inner_margin(5.0),
            |ui| {
                // Frequency center
                // ADSR knobs
                Frame::new().inner_margin(5.).show(ui, |ui| {
                    ui.add(
                        knob_default(Knob::new(
                            &mut self.lock_params().attack_duration,
                            0.0,
                            2.0,
                            KnobStyle::Wiper,
                        ))
                        .with_size(50.0)
                        .with_label("Attack", LabelPosition::Bottom),
                    );

                    ui.add(
                        knob_default(Knob::new(
                            &mut self.lock_params().decay_duration,
                            0.0,
                            10.0,
                            KnobStyle::Wiper,
                        ))
                        .with_size(50.0)
                        .with_label("Decay", LabelPosition::Bottom),
                    );
                    ui.add(
                        knob_default(Knob::new(
                            &mut self.lock_params().sustain_level,
                            0.0,
                            1.0,
                            KnobStyle::Wiper,
                        ))
                        .with_size(50.0)
                        .with_label("Sustain", LabelPosition::Bottom),
                    );
                    ui.add(
                        knob_default(Knob::new(
                            &mut self.lock_params().release_duration,
                            0.0,
                            10.0,
                            KnobStyle::Wiper,
                        ))
                        .with_size(50.0)
                        .with_label("Release", LabelPosition::Bottom),
                    );
                    // Threshold knob
                    ui.add(
                        knob_default(Knob::new(
                            &mut self.lock_params().gate_activation_threshold,
                            0.0,
                            1.0,
                            KnobStyle::Wiper,
                        ))
                            .with_size(30.0)
                            .with_label("Threshold", LabelPosition::Bottom),
                    );
                });
            },
        );
    }

    fn draw_spectrum(&mut self, ui: &mut Ui) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!("ADSREditor::draw_spectrum");
        let size = vec2(ui.available_width(), 190.);
        let mut spectrum_rect = Rect::from_min_size(ui.cursor().min, size);
        let spectrum = Image::new(SizedTexture::new(self.spectrum_texture_id.0, size));
        ui.add(spectrum);

        let lower_f = self.f_center - self.f_radius;
        let upper_f = self.f_center + self.f_radius;
        let max_frequency_range = 0.0..=MAX_FREQ;

        let ui_position_range = spectrum_rect.left()..=spectrum_rect.right();
        let lower_x = remap_clamp(lower_f, max_frequency_range.clone(), ui_position_range.clone());
        let upper_x = remap_clamp(upper_f, max_frequency_range.clone(), ui_position_range.clone());
        spectrum_rect.min.x = lower_x;
        spectrum_rect.max.x = upper_x;


        ui.painter().rect_filled(spectrum_rect, 0., Color32::from_white_alpha(150));

        scoped_frame(
            ui,
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    ui.cursor().min,
                    vec2(ui.available_width(), 100.),
                ))
                .layout(Layout::left_to_right(Align::Max)),
            Frame::default().inner_margin(5.0),
            |ui| {
                #[cfg(feature = "profiling")]
                puffin::profile_function!("ADSREditor::draw_spectrum_knobs");
                // Frequency center
                ui.add(
                    knob_default(Knob::new(&mut self.f_center, 0., 20_000., KnobStyle::Wiper))
                        .with_size(50.0)
                        .with_label("Frequency", LabelPosition::Bottom),
                );
                // Frequency radius
                ui.add(
                    knob_default(Knob::new(&mut self.f_radius, 0., 20_000., KnobStyle::Wiper))
                        .with_size(50.0)
                        .with_label("Range", LabelPosition::Bottom),
                );
                // Sensitivity knob
                ui.add(
                    knob_default(Knob::new(
                        &mut self.lock_params().sensitivity,
                        0.1,
                        10.0,
                        KnobStyle::Wiper,
                    ))
                    .with_size(50.0)
                    .with_label("Sensitivity", LabelPosition::Bottom),
                );
                self.lock_params()
                    .set_filter_tune(self.f_center, self.f_radius);
            },
        );
    }
    fn draw_curve(&self, ui: &mut Ui, rect: Rect, output_level: f32) {
        let n = 300;

        let to_screen =
            emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0), rect);

        let mut level_rect = rect.clone();
        level_rect.min.y += (1.0 - (output_level)) * rect.height();
        ui.painter()
            .rect_filled(level_rect, 0., Color32::LIGHT_GREEN);

        let mut adsr = ReactiveSignal::new(self.lock_params().clone(), 100.);
        let points: Vec<Pos2> = (0..=n)
            .map(|i| {
                let t = i as f32 / (n as f32);
                let input = if 0.2 < t
                    && t < (0.4
                        + adsr.params.decay_duration * 0.4
                        + adsr.params.attack_duration * 0.4)
                {
                    1.0
                } else {
                    0.0
                };
                let y = -1. * adsr.tick_adsr(input) + 1.;
                to_screen * pos2(t as f32, y)
            })
            .collect();

        let thickness = 1.0;
        let shape = epaint::Shape::line(points, PathStroke::new(thickness, Color32::WHITE));

        ui.painter().add(shape);
    }
    }

fn knob_default(knob: Knob) -> Knob {
    knob.with_font_size(12.0)
        .with_colors(egui::Color32::GRAY, Color32::WHITE, Color32::WHITE)
        .with_stroke_width(3.0)
}

fn draw_cursor_area(ui: &mut Ui) {
    ui.painter()
        .rect_filled(ui.cursor(), 0., Color32::PLACEHOLDER);
}
