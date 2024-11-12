use super::Effect;
use crate::{
    animation::{AnimationConfig, AnimationRenderer},
    group::Group,
    output_mix::OutputMix,
    texture_to_output::TextureToOutput,
    wgpu_render_state,
};
use egui::TextureId;

#[derive(Debug)]
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

    pub sent_group: Option<Group>,
    pub texture_to_output: TextureToOutput,
    pub texture_id: OwnedTextureId,
    pub output_mix: OutputMix,
    pub renderer: AnimationRenderer,
}

impl EffectState {
    pub fn new(effect: &Effect) -> Self {
        let output_mix = OutputMix::new();
        let (renderer, texture_to_output, texture_id) = effect.into();

        Self {
            beat_progression: 0.0,
            beats_per_minute: 0.0,
            framerate: 0.0,
            opacity: 0.0,
            color_shift: 0.0,
            sent_group: None,
            texture_to_output,
            texture_id,
            output_mix,
            renderer,
        }
    }

    pub fn update(&mut self, effect: &Effect) {
        let (renderer, texture_to_output, texture_id) = effect.into();
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

    /// must be aligned by 16 bytes
    pub fn write_data(&self, data: &mut [u8]) {
        data[0..4].copy_from_slice(&self.beat_progression.to_le_bytes());
        data[4..8].copy_from_slice(&self.beats_per_minute.to_le_bytes());
        data[8..12].copy_from_slice(&self.framerate.to_le_bytes());
        data[12..16].copy_from_slice(&self.opacity.to_le_bytes());
        data[16..20].copy_from_slice(&self.color_shift.to_le_bytes());
    }

    /// must be a multiple of 16
    pub const fn size() -> usize {
        32
    }
}

#[derive(Debug)]
pub struct OwnedTextureId(pub TextureId);

impl Drop for OwnedTextureId {
    fn drop(&mut self) {
        wgpu_render_state().renderer.write().free_texture(&self.0)
    }
}

impl From<&Effect> for (AnimationRenderer, TextureToOutput, OwnedTextureId) {
    fn from(effect: &Effect) -> Self {
        let animation_shader = effect.animation.shader_code();
        let renderer = AnimationRenderer::new(&animation_shader);
        let texture_to_output = TextureToOutput::init(renderer.texture());
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
}
