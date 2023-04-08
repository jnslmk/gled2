use crate::{
    animation::Animation,
    texture_to_artnet::{Positions, TextureToArtnet},
};
use wgpu::{Buffer, CommandEncoder, Device, Queue};

pub struct Scene {
    animation: Animation,
    texture_to_artnet: TextureToArtnet,
}

impl Scene {
    pub fn new(device: &Device, animation: Animation, positions: &Positions) -> Self {
        let texture_to_artnet =
            TextureToArtnet::init(device, animation.renderer().texture(), positions);

        Self {
            animation,
            texture_to_artnet,
        }
    }

    pub fn prepare(
        &self,
        queue: &Queue,
        beat_progression: f32,
        beats_per_minute: f32,
        framerate: f32,
    ) {
        self.animation
            .renderer()
            .prepare(queue, beat_progression, beats_per_minute, framerate);
    }

    pub fn render(&self, encoder: &mut CommandEncoder) {
        self.animation.renderer().render(encoder);
        self.texture_to_artnet.run(encoder);
    }

    pub fn artnet_buffer(&self) -> &Buffer {
        self.texture_to_artnet.artnet_buffer()
    }
}
