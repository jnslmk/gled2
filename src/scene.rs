use crate::{
    animation::{Animation, ColorPalette, State},
    app::positions,
    texture_to_artnet::TextureToArtnet,
    wgpu_render_state,
};
use egui::TextureId;
use serde::{Deserialize, Serialize};
use wgpu::{Buffer, CommandEncoder, Queue};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Scene {
    #[serde(skip)]
    texture_to_artnet: Option<TextureToArtnet>,
    animation: Animation,
    pub palette: ColorPalette,
    pub opacity: f32,
    pub artnet_extraction: bool,
    group: String,
    sent_group: Option<String>,
    #[serde(skip)]
    texture_id: Option<TextureId>,
    #[serde(skip)]
    artnet_dirty: bool,
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

    pub fn prepare(
        &mut self,
        queue: &Queue,
        mut state: State,
        disable_artnet_extraction: bool,
        main_dimmer: f32,
    ) {
        // make sure we have a texture id (fixes a deadlock).
        self.texture_id();

        state.opacity = self.opacity * main_dimmer;

        if self.sent_group.as_ref() != Some(&self.group) {
            let positions = positions(&self.group);
            self.texture_to_artnet().set_positions(queue, positions);
            self.sent_group = Some(self.group.clone());
        }

        self.animation
            .renderer()
            .set_buffers(queue, &state, &self.palette);

        if (!self.artnet_extraction || disable_artnet_extraction) && self.artnet_dirty {
            self.texture_to_artnet().clear_artnet(queue);
            self.artnet_dirty = false;
        }
    }

    pub fn render(&mut self, encoder: &mut CommandEncoder, disable_artnet_extraction: bool) {
        self.animation.renderer().render(encoder);
        if self.artnet_extraction && !disable_artnet_extraction {
            self.texture_to_artnet().run(encoder);
            self.artnet_dirty = true;
        }
    }

    pub fn artnet_buffer(&mut self) -> &Buffer {
        self.texture_to_artnet().artnet_buffer()
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

    pub fn texture_to_artnet(&mut self) -> &TextureToArtnet {
        self.texture_to_artnet
            .as_ref()
            .expect("Gpu was not yet initialized")
    }

    pub fn texture_id(&mut self) -> TextureId {
        self.texture_id.expect("Gpu was not yet initialized")
    }
}
