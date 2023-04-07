use crate::{
    animation::Animation,
    texture_to_artnet::{Positions, TextureToArtnet},
};
use wgpu::{Buffer, Device, Queue};

pub struct Scene {
    animation: Animation,
    texture_to_artnet: TextureToArtnet,
}

impl Scene {
    pub fn new(device: &Device, animation: Animation, positions: &Positions) -> Self {
        let texture_to_artnet = TextureToArtnet::init(device, animation.texture(), positions);

        Self {
            animation,
            texture_to_artnet,
        }
    }
    pub fn render(&self, device: &Device, queue: &Queue) {
        self.animation.prepare(queue);
        self.animation.render(device, queue);
        self.texture_to_artnet.run(device, queue);
    }

    pub fn artnet_buffer(&self) -> &Buffer {
        self.texture_to_artnet.artnet_buffer()
    }
}
