use std::time::Instant;

use crate::{
    animation::State, extract_artnet::ExtractArtnet, mix_artnet::MixArtnet, scene::Scene,
    svg::Universes,
};
use artnet_protocol::ArtCommand;
use slab::Slab;
use wgpu::{CommandEncoderDescriptor, Device, Queue};

pub struct Pipeline {
    start: Instant,
    scenes: Slab<Scene>,
    mixs: Vec<MixArtnet>,
    extract: ExtractArtnet,
}

impl Pipeline {
    pub fn init() -> Self {
        let extract = ExtractArtnet::init();

        Self {
            start: Instant::now(),
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

    pub fn set_opacity(&mut self, index: usize, opacity: f32) {
        if let Some(scene) = self.scenes.get_mut(index) {
            scene.opacity = opacity;
        }
    }

    pub fn set_artnet_extraction(&mut self, index: usize, artnet_extraction: bool) {
        if let Some(scene) = self.scenes.get_mut(index) {
            scene.artnet_extraction = artnet_extraction;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_and_poll(
        &mut self,
        device: &Device,
        queue: &Queue,
        universes: &Universes,
        beat_progression: f32,
        beats_per_minute: f32,
        framerate: f32,
        disable_artnet_extraction: bool,
    ) -> Option<Vec<ArtCommand>> {
        let time = self.start.elapsed().as_secs_f32();
        let state = State {
            time,
            beat_progression,
            beats_per_minute,
            framerate,
            ..Default::default()
        };

        for (_index, scene) in self.scenes.iter_mut() {
            scene.prepare(queue, state);
        }

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });
        for (_index, scene) in self.scenes.iter() {
            scene.render(&mut encoder, disable_artnet_extraction);
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
            Some(self.extract.poll_artnet_buffer(universes))
        } else {
            None
        }
    }

    pub fn scenes(&mut self) -> Vec<&mut Scene> {
        self.scenes
            .iter_mut()
            .map(|(_index, scene)| scene)
            .collect()
    }
}

#[macro_export]
macro_rules! get_pipeline {
    ($field: ident) => {
        let wgpu_render_state = wgpu_render_state();
        let mut renderer = wgpu_render_state.renderer.write();
        let $field = renderer
            .paint_callback_resources
            .get_mut::<Pipeline>()
            .expect("Could not find Pipeline");
    };
}
