use crate::{extract_artnet::ExtractArtnet, mix_artnet::MixArtnet, scene::Scene};
use slab::Slab;
use wgpu::{CommandEncoderDescriptor, Device, Queue};

pub struct Pipeline {
    scenes: Slab<Scene>,
    mixs: Vec<MixArtnet>,
    extract: ExtractArtnet,
}

impl Pipeline {
    pub fn init(device: &Device) -> Self {
        let extract = ExtractArtnet::init(device);

        Self {
            scenes: Slab::new(),
            mixs: Vec::new(),
            extract,
        }
    }

    pub fn add_scene(&mut self, scene: Scene) -> usize {
        self.scenes.insert(scene)
    }

    pub fn remove_scene(&mut self, index: usize) {
        self.scenes.remove(index);
    }

    pub fn run_and_poll(&mut self, device: &Device, queue: &Queue) -> Option<Vec<u8>> {
        for (_index, scene) in self.scenes.iter() {
            scene.prepare(queue);
        }

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });
        for (_index, scene) in self.scenes.iter() {
            scene.render(&mut encoder);
        }

        let mut main = None;
        let mut mix_index = 0;
        for (_index, scene) in self.scenes.iter() {
            match main {
                Some(main) => {
                    if self.mixs.len() <= mix_index {
                        self.mixs.push(MixArtnet::init(device));
                    }
                    self.mixs.get(mix_index).expect("Mix does not exist").run(
                        device,
                        &mut encoder,
                        main,
                        scene.artnet_buffer(),
                    );
                    mix_index += 1;
                }
                None => main = Some(scene.artnet_buffer()),
            }
        }

        if let Some(main) = main {
            self.extract.run(&mut encoder, main);
        }
        queue.submit(std::iter::once(encoder.finish()));

        if main.is_some() {
            Some(self.extract.poll_artnet_buffer(device))
        } else {
            None
        }
    }
}
