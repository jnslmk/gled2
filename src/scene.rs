use crate::{
    animation::{Animation, ColorPalette, State},
    app::positions,
    texture_to_artnet::TextureToArtnet,
};
use wgpu::{Buffer, CommandEncoder, Device, Queue};

pub struct Scene {
    texture_to_artnet: TextureToArtnet,
    animation: Animation,
    pub palette: ColorPalette,
    pub opacity: f32,
    pub artnet_extraction: bool,
    group: String,
    /// Resend positions to the gpu
    send_positions: bool,
}

impl Scene {
    pub fn new(
        device: &Device,
        animation: Animation,
        palette: ColorPalette,
        group: String,
    ) -> Self {
        let texture_to_artnet = TextureToArtnet::init(device, animation.renderer().texture());

        Self {
            texture_to_artnet,
            animation,
            palette,
            opacity: 1.0,
            artnet_extraction: false,
            group,
            send_positions: true,
        }
    }

    pub fn prepare(&mut self, queue: &Queue, mut state: State) {
        state.opacity = self.opacity;

        if self.send_positions {
            self.texture_to_artnet
                .set_positions(queue, positions(&self.group).unwrap_or_default());
            self.send_positions = false;
        }

        self.animation
            .renderer()
            .set_buffers(queue, &state, &self.palette);
    }

    pub fn render(&self, encoder: &mut CommandEncoder, disable_artnet_extraction: bool) {
        self.animation.renderer().render(encoder);
        if self.artnet_extraction && !disable_artnet_extraction {
            self.texture_to_artnet.run(encoder);
        }
    }

    pub fn artnet_buffer(&self) -> &Buffer {
        self.texture_to_artnet.artnet_buffer()
    }

    pub fn group(&self) -> &str {
        &self.group
    }

    pub fn set_group(&mut self, group: String) {
        self.group = group;
        self.send_positions();
    }

    pub fn send_positions(&mut self) {
        self.send_positions = true;
    }
}
