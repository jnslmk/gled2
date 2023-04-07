use crate::{animation::Animation, texture_to_artnet::TextureToArtnet};
use wgpu::{Device, Queue};

pub struct Scene {
    pub animation: Animation,
    pub texture_to_artnet: TextureToArtnet,
}

impl Scene {
    pub fn render(&self, device: &Device, queue: &Queue) {
        self.animation.prepare(queue);
        self.animation.render(device, queue);
        self.texture_to_artnet.run(device, queue);
    }
}
