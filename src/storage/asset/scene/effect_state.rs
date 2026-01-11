use super::Effect;
use crate::{
    pipeline::{
        group::Group, output_mix::OutputMix, renderer_callback::RendererCallback,
        texture_to_output::TextureToOutput,
    },
    storage::animation::{config::AnimationConfig, renderer::AnimationRenderer},
    ui::action::UiAction,
    wgpu_render_state,
};
use arboard::{Clipboard, ImageData};
use egui::TextureId;
use rand::Rng;
use wgpu::{MapMode, PollType};

#[derive(Debug, PartialEq)]
pub struct EffectState {
    /// Progress in current beat.
    pub beat_progression: f32,
    /// Beats per minute
    pub beats_per_minute: f32,
    /// Current displayed framerate
    pub framerate: f32,
    /// Opacity of animation: 0.0 -> 1.0
    pub opacity: f32,
    /// Color shift in full circles. (0.5 = 180 degrees)
    pub color_shift: f32,
    /// In 2^n of bpm
    pub speed_exponent: i32,

    pub animation_config: AnimationConfig,

    pub sent_group: Option<Group>,
    pub texture_to_output: TextureToOutput,
    pub texture_id: OwnedTextureId,
    pub output_mix: OutputMix,
    pub renderer: AnimationRenderer,
}

impl EffectState {
    pub fn new(effect: &Effect) -> Self {
        let (renderer, texture_to_output, texture_id) =
            setup_pipeline(&effect.shader_code_complete());

        let output_mix = OutputMix::new();
        Self {
            beat_progression: 0.0,
            beats_per_minute: 0.0,
            framerate: 0.0,
            opacity: 0.0,
            color_shift: 0.0,
            speed_exponent: 0,
            animation_config: Default::default(),
            sent_group: None,
            texture_to_output,
            texture_id,
            output_mix,
            renderer,
        }
    }

    pub fn update(&mut self, effect: &Effect) {
        let (renderer, texture_to_output, texture_id) =
            setup_pipeline(&effect.shader_code_complete());
        self.renderer = renderer;
        self.texture_to_output = texture_to_output;
        self.texture_id = texture_id;
    }

    /// Resend positions to gpu
    pub fn send_positions(&mut self) {
        self.sent_group.take();
    }

    pub fn texture_id(&self) -> TextureId {
        self.texture_id.0
    }

    pub fn write_data(&self, beat_progression: f32, data: &mut [u8]) {
        data[0..4].copy_from_slice(&rand::rng().random::<f32>().to_le_bytes());
        data[4..8].copy_from_slice(&self.beat_progression.to_le_bytes());
        data[8..12].copy_from_slice(&self.beats_per_minute.to_le_bytes());
        data[12..16].copy_from_slice(&self.framerate.to_le_bytes());
        data[16..20].copy_from_slice(&self.opacity.to_le_bytes());
        data[20..24].copy_from_slice(&self.color_shift.to_le_bytes());
        data[24..28].copy_from_slice(&2f32.powi(self.speed_exponent).to_le_bytes());
        self.animation_config.write_data(
            &mut data[28..28 + AnimationConfig::size()],
            beat_progression,
        );

        // Write FFT data (256 frequency bins = 1024 bytes)
        // TODO receive updates again
        // let fft_data = fft_data_u8();
        let fft_data = [0u8; 256 * 4];
        let fft_offset = 28 + AnimationConfig::size();
        data[fft_offset..fft_offset + fft_data.len()].copy_from_slice(&fft_data);
    }

    pub const fn size() -> usize {
        const SIZE: usize = 28 + AnimationConfig::size() + 256 * 4; // 64 + 1024 = 1088 bytes
        static_assertions::const_assert_eq!(SIZE % 16, 0);
        SIZE
    }

    pub fn copy_rendered_image_to_clipboard(&self) {
        let texture = self.renderer.texture();
        let texture_size = texture.size();
        let buffer_size = (texture_size.width * texture_size.height * 4) as wgpu::BufferAddress;
        let buffer = wgpu_render_state()
            .device
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("Texture Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
        let mut encoder =
            wgpu_render_state()
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Copy Texture to Buffer"),
                });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: self.renderer.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(texture_size.width * 4),
                    rows_per_image: Some(texture_size.height),
                },
            },
            texture_size,
        );

        RendererCallback::add(encoder.finish());

        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(MapMode::Read, move |v| {
            tx.send(v).expect("Could not send on oneshot sender")
        });
        wgpu_render_state()
            .device
            .poll(PollType::wait_indefinitely())
            .expect("Could not poll device");
        rx.recv()
            .expect("Could not receive on gpu rx")
            .expect("Error receiving answer to output_data map on gpu");
        let mut data = buffer_slice.get_mapped_range().to_vec();
        buffer.unmap();

        std::thread::spawn(move || {
            #[cfg(feature = "profiling")]
            profiling::register_thread!("copy_image_to_clipboard");

            data.chunks_exact_mut(4).for_each(|pixel| {
                pixel.swap(0, 2);
                pixel[3] = 255;
            });

            if let Ok(mut clipboard) = Clipboard::new() {
                if let Err(err) = clipboard.set_image(ImageData {
                    bytes: data.into(),
                    width: texture_size.width as usize,
                    height: texture_size.height as usize,
                }) {
                    UiAction::Error(format!("Could not copy rendered image to clipboard: {err}"))
                        .enqueue();
                } else {
                    log::info!("Copied rendered image to clipboard");
                }
            }
        });
    }
}

#[derive(Debug, PartialEq)]
pub struct OwnedTextureId(pub TextureId);

impl Drop for OwnedTextureId {
    fn drop(&mut self) {
        wgpu_render_state().renderer.write().free_texture(&self.0)
    }
}

fn setup_pipeline(shader_code: &str) -> (AnimationRenderer, TextureToOutput, OwnedTextureId) {
    let renderer = AnimationRenderer::new(shader_code);
    let texture_to_output = TextureToOutput::init(renderer.texture(), renderer.uniforms());
    let texture_id = OwnedTextureId(
        wgpu_render_state()
            .renderer
            .write()
            .register_native_texture(
                &wgpu_render_state().device,
                renderer.view(),
                wgpu::FilterMode::Nearest,
            ),
    );

    (renderer, texture_to_output, texture_id)
}
