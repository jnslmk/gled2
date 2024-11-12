use crate::{
    animation::{Animation, AnimationConfig, AnimationRenderer, State},
    app::{positions, PersistantState},
    constants::GPU_NOT_INIT,
    input::InputEvent,
    output_mix::OutputMix,
    storage::{Asset, Palette},
    texture_to_output::TextureToOutput,
    transition::Transition,
    wgpu_render_state,
};
use egui::TextureId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{Buffer, CommandEncoder, Queue};

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Effect {
    pub color_shift: f32,
    pub opacity: f32,
    pub active: bool,
    pub beat_progression_offset: f32,
    pub group: String,
    pub animation: Animation,
    pub selection_input: Option<InputEvent>,
    pub flash_input: Option<InputEvent>,
    pub dimmer_input: Option<InputEvent>,
    pub flash: bool,
    pub input_dimmer: f32,

    #[serde(skip)]
    transition: Option<Transition>,
    #[serde(skip)]
    sent_group: Option<String>,
    #[serde(skip)]
    texture_to_output: Option<TextureToOutput>,
    #[serde(skip)]
    texture_id: Option<OwnedTextureId>,
    #[serde(skip)]
    output_is_dirty: bool,
    #[serde(skip)]
    was_ever_rendered: bool,
    #[serde(skip)]
    output_mix: Option<OutputMix>,
    #[serde(skip)]
    renderer: Option<AnimationRenderer>,
}

impl PartialEq for Effect {
    fn eq(&self, other: &Self) -> bool {
        self.color_shift == other.color_shift
            && self.opacity == other.opacity
            && self.active == other.active
            && self.beat_progression_offset == other.beat_progression_offset
            && self.group == other.group
            && self.animation == other.animation
            && self.selection_input == other.selection_input
            && self.flash_input == other.flash_input
            && self.flash == other.flash
            && self.input_dimmer == other.input_dimmer
    }
}

impl Eq for Effect {}

impl Clone for Effect {
    fn clone(&self) -> Self {
        Self {
            color_shift: self.color_shift,
            opacity: self.opacity,
            active: self.active,
            beat_progression_offset: self.beat_progression_offset,
            group: self.group.clone(),
            animation: self.animation.clone(),
            selection_input: self.selection_input,
            flash_input: self.flash_input,
            ..Default::default()
        }
    }
}

impl Effect {
    pub fn new(animation: Animation, group: String) -> Self {
        Self {
            animation,
            opacity: 1.0,
            active: true,
            group,
            input_dimmer: 1.0,
            ..Default::default()
        }
    }

    pub fn init_gpu(&mut self) {
        self.output_mix.get_or_insert_with(OutputMix::new);
        self.renderer.get_or_insert_with(|| {
            let animation_shader = self.animation.shader_code();
            AnimationRenderer::new(&animation_shader)
        });
        self.texture_to_output.get_or_insert_with(|| {
            TextureToOutput::init(self.renderer.as_ref().expect(GPU_NOT_INIT).texture())
        });
        self.texture_id.get_or_insert_with(|| {
            OwnedTextureId(
                wgpu_render_state()
                    .renderer
                    .write()
                    .register_native_texture(
                        &wgpu_render_state().device,
                        self.renderer.as_ref().expect(GPU_NOT_INIT).view(),
                        wgpu::FilterMode::Nearest,
                    ),
            )
        });
    }

    pub fn reset_gpu_state(&mut self) {
        self.sent_group.take();
        self.texture_id.take();
        self.texture_to_output.take();
        self.renderer.take();
        self.output_mix.take();
        self.init_gpu();
    }

    pub fn set_buffers(&mut self, main: &Buffer) {
        let other = self
            .texture_to_output
            .as_ref()
            .expect(GPU_NOT_INIT)
            .output_buffer();
        self.output_mix
            .as_mut()
            .expect(GPU_NOT_INIT)
            .set_buffers(main, other);
    }

    pub fn prepare(
        &mut self,
        queue: &Queue,
        mut state: State,
        blackout: bool,
        always_render: bool,
        palette: Option<Arc<Asset<Palette>>>,
    ) {
        // make sure we have a texture id (fixes a deadlock).
        self.texture_id();

        let mut opacity_factor = 1.0;
        if let Some(transition) = self.transition.as_ref() {
            match transition.opacity_factor() {
                Some(factor) => opacity_factor = factor,
                None => {
                    if transition.goal().turning_off() {
                        self.active = false;
                        self.was_ever_rendered = false;
                    }
                    self.transition.take();
                }
            }
        }

        let main_dimmer = PersistantState::main_dimmer();

        if always_render || self.active || !self.was_ever_rendered || self.flash {
            state.opacity = self.opacity * main_dimmer * opacity_factor * self.input_dimmer;
            state.beat_progression += self.beat_progression_offset;
            state.color_shift = self.color_shift;

            if self.sent_group.as_ref() != Some(&self.group) {
                let positions = positions(&self.group);
                self.texture_to_output
                    .as_ref()
                    .expect(GPU_NOT_INIT)
                    .set_positions(queue, positions);
                self.sent_group = Some(self.group.clone());
            }

            self.renderer.as_ref().expect(GPU_NOT_INIT).set_buffers(
                queue,
                &state,
                palette,
                &self.animation.config(),
            );

            if (!self.active || blackout) && self.output_is_dirty {
                self.texture_to_output
                    .as_ref()
                    .expect(GPU_NOT_INIT)
                    .clear_output(queue);
                self.output_is_dirty = false;
            }
        }
    }

    pub fn render(&mut self, encoder: &mut CommandEncoder, blackout: bool, always_render: bool) {
        if always_render || self.active || !self.was_ever_rendered || self.flash {
            self.renderer.as_ref().expect(GPU_NOT_INIT).render(encoder);
            if (self.active && !blackout) || self.flash {
                self.texture_to_output
                    .as_ref()
                    .expect(GPU_NOT_INIT)
                    .run(encoder);
                self.output_is_dirty = true;
                self.output_mix.as_ref().expect(GPU_NOT_INIT).run(encoder);
            }
            self.was_ever_rendered = true;
        }
    }

    /// Resend positions to gpu
    pub fn send_positions(&mut self) {
        self.sent_group = None;
    }

    pub fn texture_id(&self) -> TextureId {
        self.texture_id.as_ref().expect(GPU_NOT_INIT).0
    }

    pub fn set_transition(&mut self, transition: Transition) {
        self.active = true;
        self.transition = Some(transition);
    }

    pub fn has_transition(&self) -> bool {
        self.transition.is_some()
    }

    pub fn transition_factor(&self) -> f32 {
        self.transition
            .as_ref()
            .and_then(|transition| transition.opacity_factor())
            .unwrap_or(1.0)
    }

    pub fn set_flash(&mut self, flash: bool) {
        self.flash = flash;
    }

    pub fn set_input_dimmer(&mut self, input_dimmer: f32) {
        self.input_dimmer = input_dimmer;
    }
}

#[derive(Debug)]
struct OwnedTextureId(pub TextureId);

impl Drop for OwnedTextureId {
    fn drop(&mut self) {
        wgpu_render_state().renderer.write().free_texture(&self.0)
    }
}
