use crate::{animation::Animation, extract::Extract};
use wgpu::{Device, Queue};

pub struct Scene {
    pub animation: Animation,
    pub extract: Extract,
}

impl Scene {
    pub fn render(&self, device: &Device, queue: &Queue) -> Vec<u8> {
        self.animation.prepare(queue);
        self.animation.render(device, queue);

        self.extract.run_and_poll(device, queue)
    }
}
