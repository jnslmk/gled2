use crate::{
    animation::{Animation, ColorPalette, State},
    texture_to_artnet::{Positions, TextureToArtnet},
};
use wgpu::{Buffer, CommandEncoder, Device, Queue};

pub struct Scene {
    texture_to_artnet: TextureToArtnet,
    animation: Animation,
    pub palette: ColorPalette,
    pub opacity: f32,
    pub artnet_extraction: bool,
}

impl Scene {
    pub fn new(
        device: &Device,
        animation: Animation,
        palette: ColorPalette,
        positions: &Positions,
    ) -> Self {
        let texture_to_artnet =
            TextureToArtnet::init(device, animation.renderer().texture(), positions);

        Self {
            texture_to_artnet,
            animation,
            palette,
            opacity: 1.0,
            artnet_extraction: false,
        }
    }

    pub fn prepare(&self, queue: &Queue, mut state: State) {
        state.opacity = self.opacity;

        self.animation
            .renderer()
            .prepare(queue, &state, &self.palette);
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
}
