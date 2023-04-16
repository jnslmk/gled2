use crate::{
    animation::{Animation, ColorPalette, State},
    app::positions,
    artnet_mix::ArtnetMix,
    constants::GPU_NOT_INIT,
    texture_to_artnet::TextureToArtnet,
    wgpu_render_state,
};
use egui::TextureId;
use serde::{Deserialize, Serialize};
use wgpu::{Buffer, CommandEncoder, Queue};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Scene {
    pub kind: SceneKind,
    animation: Animation,
    pub palette: ColorPalette,
    pub opacity: f32,
    pub artnet_extraction: bool,
    pub beat_progression_offset: f32,
    group: String,

    #[serde(skip)]
    sent_group: Option<String>,
    #[serde(skip)]
    texture_to_artnet: Option<TextureToArtnet>,
    #[serde(skip)]
    texture_id: Option<TextureId>,
    #[serde(skip)]
    artnet_dirty: bool,
    #[serde(skip)]
    was_ever_rendered: bool,
    #[serde(skip)]
    artnet_mix: Option<ArtnetMix>,
}

impl Clone for Scene {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind,
            animation: self.animation.clone(),
            palette: self.palette.clone(),
            opacity: self.opacity,
            artnet_extraction: self.artnet_extraction,
            beat_progression_offset: self.beat_progression_offset,
            group: self.group.clone(),
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
            artnet_extraction: true,
            group,
            ..Default::default()
        }
    }

    pub fn init_gpu(&mut self) {
        self.artnet_mix.get_or_insert_with(ArtnetMix::new);
        self.animation.init_gpu();
        self.texture_to_artnet
            .get_or_insert_with(|| TextureToArtnet::init(self.animation.renderer().texture()));
        self.texture_id.get_or_insert_with(|| {
            wgpu_render_state()
                .renderer
                .write()
                .register_native_texture(
                    &wgpu_render_state().device,
                    self.animation.renderer().view(),
                    wgpu::FilterMode::Nearest,
                )
        });
    }

    pub fn set_buffers(&mut self, main: &Buffer) {
        let ours = self
            .texture_to_artnet
            .as_ref()
            .expect(GPU_NOT_INIT)
            .artnet_buffer();
        self.artnet_mix
            .as_mut()
            .expect(GPU_NOT_INIT)
            .set_buffers(main, ours)
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

        if always_render || self.artnet_extraction || !self.was_ever_rendered {
            state.opacity = self.opacity * main_dimmer;
            state.beat_progression = (state.beat_progression + self.beat_progression_offset) % 1.0;

            if self.sent_group.as_ref() != Some(&self.group) {
                let positions = positions(&self.group);
                self.texture_to_artnet
                    .as_ref()
                    .expect(GPU_NOT_INIT)
                    .set_positions(queue, positions);
                self.sent_group = Some(self.group.clone());
            }

            self.animation
                .renderer()
                .set_buffers(queue, &state, &self.palette);

            if (!self.artnet_extraction || blackout) && self.artnet_dirty {
                self.texture_to_artnet
                    .as_ref()
                    .expect(GPU_NOT_INIT)
                    .clear_artnet(queue);
                self.artnet_dirty = false;
            }
        }
    }

    pub fn render(&mut self, encoder: &mut CommandEncoder, blackout: bool, always_render: bool) {
        if always_render || self.artnet_extraction || !self.was_ever_rendered {
            self.animation.renderer().render(encoder);
            if self.artnet_extraction && !blackout {
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

    pub fn group(&self) -> &str {
        &self.group
    }

    pub fn group_mut(&mut self) -> &mut String {
        &mut self.group
    }

    /// Resend positions to gpu
    pub fn send_positions(&mut self) {
        self.sent_group = None;
    }

    pub fn texture_id(&self) -> TextureId {
        self.texture_id.expect(GPU_NOT_INIT)
    }
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneKind {
    #[default]
    Background,
    Foreground,
}
