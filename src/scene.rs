use crate::{
    animation::{Animation, AnimationConfig, AnimationRenderer, ColorPalette, State},
    app::positions,
    artnet_mix::ArtnetMix,
    constants::GPU_NOT_INIT,
    texture_to_artnet::TextureToArtnet,
    transition::Transition,
    wgpu_render_state,
};
use egui::{Key, TextureId};
use serde::{Deserialize, Serialize};
use wgpu::{Buffer, CommandEncoder, Queue};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Scene {
    pub kind: SceneKind,
    pub palette: ColorPalette,
    pub opacity: f32,
    pub active: bool,
    pub beat_progression_offset: f32,
    pub group: String,
    pub animation: Animation,
    pub key: Option<Key>,

    #[serde(skip)]
    transition: Option<Transition>,
    #[serde(skip)]
    sent_group: Option<String>,
    #[serde(skip)]
    texture_to_artnet: Option<TextureToArtnet>,
    #[serde(skip)]
    texture_id: Option<OwnedTextureId>,
    #[serde(skip)]
    artnet_dirty: bool,
    #[serde(skip)]
    was_ever_rendered: bool,
    #[serde(skip)]
    artnet_mix: Option<ArtnetMix>,
    #[serde(skip)]
    renderer: Option<AnimationRenderer>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            key: Default::default(),
            kind: Default::default(),
            palette: Default::default(),
            active: Default::default(),
            beat_progression_offset: Default::default(),
            group: Default::default(),
            animation: Default::default(),
            transition: Default::default(),
            sent_group: Default::default(),
            texture_to_artnet: Default::default(),
            texture_id: Default::default(),
            artnet_dirty: Default::default(),
            was_ever_rendered: Default::default(),
            artnet_mix: Default::default(),
            renderer: Default::default(),
        }
    }
}

impl Clone for Scene {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind,
            palette: self.palette.clone(),
            opacity: self.opacity,
            active: self.active,
            beat_progression_offset: self.beat_progression_offset,
            group: self.group.clone(),
            animation: self.animation.clone(),
            key: self.key,
            ..Default::default()
        }
    }
}

fn default_send_positions() -> bool {
    true
}

impl Scene {
    pub fn new(animation: Animation, palette: ColorPalette, group: String) -> Self {
        Self {
            animation,
            palette,
            opacity: 1.0,
            active: true,
            group,
            ..Default::default()
        }
    }

    pub fn init_gpu(&mut self) {
        self.artnet_mix.get_or_insert_with(ArtnetMix::new);
        self.renderer.get_or_insert_with(|| {
            let config = self.animation.config();
            let animation_shader = self.animation.shader_code();
            AnimationRenderer::new(&animation_shader, &config)
        });
        self.texture_to_artnet.get_or_insert_with(|| {
            TextureToArtnet::init(self.renderer.as_ref().expect(GPU_NOT_INIT).texture())
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
        self.texture_to_artnet.take();
        self.renderer.take();
        self.artnet_mix.take();
        self.init_gpu();
    }

    pub fn set_buffers(&mut self, main: &Buffer) {
        let other = self
            .texture_to_artnet
            .as_ref()
            .expect(GPU_NOT_INIT)
            .artnet_buffer();
        self.artnet_mix
            .as_mut()
            .expect(GPU_NOT_INIT)
            .set_buffers(main, other);
    }

    pub fn prepare(
        &mut self,
        queue: &Queue,
        mut state: State,
        blackout: bool,
        main_dimmer: f32,
        always_render: bool,
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

        if always_render || self.active || !self.was_ever_rendered {
            state.opacity = self.opacity * main_dimmer * opacity_factor;

            state.beat_progression = (state.beat_progression + self.beat_progression_offset) % 1.0;

            if self.sent_group.as_ref() != Some(&self.group) {
                let positions = positions(&self.group);
                self.texture_to_artnet
                    .as_ref()
                    .expect(GPU_NOT_INIT)
                    .set_positions(queue, positions);
                self.sent_group = Some(self.group.clone());
            }

            self.renderer.as_ref().expect(GPU_NOT_INIT).set_buffers(
                queue,
                &state,
                &self.palette,
                &self.animation.config(),
            );

            if (!self.active || blackout) && self.artnet_dirty {
                self.texture_to_artnet
                    .as_ref()
                    .expect(GPU_NOT_INIT)
                    .clear_artnet(queue);
                self.artnet_dirty = false;
            }
        }
    }

    pub fn render(&mut self, encoder: &mut CommandEncoder, blackout: bool, always_render: bool) {
        if always_render || self.active || !self.was_ever_rendered {
            self.renderer.as_ref().expect(GPU_NOT_INIT).render(encoder);
            if self.active && !blackout {
                self.texture_to_artnet
                    .as_ref()
                    .expect(GPU_NOT_INIT)
                    .run(encoder);
                self.artnet_dirty = true;
                self.artnet_mix.as_ref().expect(GPU_NOT_INIT).run(encoder);
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

    pub fn config_ui(&mut self, ui: &mut egui::Ui) {
        self.animation.ui(ui, self.texture_id());
    }

    pub fn set_transition(&mut self, transition: Transition) {
        self.active = true;
        self.transition = Some(transition);
    }

    pub fn has_transition(&self) -> bool {
        self.transition.is_some()
    }

    /// whether the scene is (turning) on
    pub fn on(&self) -> bool {
        self.active
            && match self.transition.as_ref() {
                None => true,
                Some(transition) => transition.goal().turning_on(),
            }
    }

    pub fn transition_factor(&self) -> f32 {
        self.transition
            .as_ref()
            .and_then(|transition| transition.opacity_factor())
            .unwrap_or(1.0)
    }
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneKind {
    #[default]
    Background,
    Foreground,
}

impl SceneKind {
    pub fn switch(&mut self) {
        *self = match self {
            Self::Background => Self::Foreground,
            Self::Foreground => Self::Background,
        }
    }
}

#[derive(Debug)]
struct OwnedTextureId(pub TextureId);

impl Drop for OwnedTextureId {
    fn drop(&mut self) {
        wgpu_render_state().renderer.write().free_texture(&self.0)
    }
}
