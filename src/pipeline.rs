use crate::{
    animation::State, artnet_sender::ArtnetSender, extract_artnet::ExtractArtnet,
    mix_artnet::MixArtnet, preview::Preview, preview_indices::PreviewIndices, scene::Scene,
    svg::Universes, wgpu_render_state,
};
use artnet_protocol::ArtCommand;
use egui::TextureId;
use serde::{Deserialize, Serialize};
use slab::Slab;
use std::time::Instant;
use wgpu::{CommandEncoderDescriptor, Device, Queue};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pipeline {
    #[serde(skip)]
    start: Option<Instant>,
    scenes: Slab<Scene>,
    #[serde(skip)]
    mixs: Vec<MixArtnet>,
    #[serde(skip)]
    extract: Option<ExtractArtnet>,
    #[serde(skip)]
    preview_indices: Option<PreviewIndices>,
    #[serde(skip)]
    preview: Option<Preview>,
}

impl Pipeline {
    pub fn init_gpu(&mut self) {
        self.extract.get_or_insert_with(ExtractArtnet::new);
        self.preview_indices.get_or_insert_with(PreviewIndices::new);
        self.preview.get_or_insert_with(Preview::new);
        for (_index, scene) in self.scenes.iter_mut() {
            scene.init_gpu();
        }
    }

    pub fn start(&mut self) -> Instant {
        *self.start.get_or_insert_with(Instant::now)
    }

    pub fn add_scene(&mut self, scene: Scene) -> usize {
        let index = self.scenes.insert(scene);
        self.init_gpu();

        index
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
    fn run_and_poll(
        &mut self,
        device: &Device,
        queue: &Queue,
        universes: &Universes,
        beat_progression: f32,
        beats_per_minute: f32,
        framerate: f32,
        disable_artnet_extraction: bool,
    ) -> Option<Vec<ArtCommand>> {
        let time = self.start().elapsed().as_secs_f32();
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
        self.preview_indices
            .as_mut()
            .expect("Gpu is not yet initialized")
            .prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });
        for (_index, scene) in self.scenes.iter_mut() {
            scene.render(&mut encoder, disable_artnet_extraction);
        }

        let mut main = None;
        let mut mix_index = 0;
        for (_index, scene) in self.scenes.iter_mut() {
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
            self.extract
                .as_mut()
                .expect("Gpu was not yet initialized")
                .run(&mut encoder, main);
            self.preview_indices
                .as_mut()
                .expect("Gpu was not yet initialized")
                .run(&mut encoder);
            self.preview
                .as_mut()
                .expect("Gpu was not yet initialized")
                .run(
                    &mut encoder,
                    self.preview_indices
                        .as_ref()
                        .expect("Gpu was not yet initialized")
                        .indices(),
                    main,
                );
        }
        queue.submit(std::iter::once(encoder.finish()));

        if main.is_some() {
            Some(
                self.extract
                    .as_mut()
                    .expect("Gpu was not yet initialized")
                    .poll_artnet_buffer(universes),
            )
        } else {
            None
        }
    }

    pub fn scenes(&mut self) -> Vec<(usize, &mut Scene)> {
        self.scenes.iter_mut().collect()
    }

    pub fn send_positions(&mut self) {
        for (_index, scene) in self.scenes.iter_mut() {
            scene.send_positions();
        }
        self.preview_indices
            .as_mut()
            .expect("Gpu was not yet initialized")
            .send_positions();
    }

    pub fn preview_texture_id(&self) -> TextureId {
        self.preview
            .as_ref()
            .expect("Gpu was not yet initialized")
            .texture_id()
    }

    pub fn render(
        &mut self,
        universes: &Universes,
        artnet_sender: &mut ArtnetSender,
        beat_progression: f32,
        beats_per_minute: f32,
        framerate: f32,
        disable_artnet_extraction: bool,
    ) {
        let wgpu_render_state = wgpu_render_state();
        let device = &wgpu_render_state.device;
        let queue = &wgpu_render_state.queue;

        let commands = self
            .run_and_poll(
                device,
                queue,
                universes,
                beat_progression,
                beats_per_minute,
                framerate,
                disable_artnet_extraction,
            )
            .expect("No scene registered");

        for command in commands {
            artnet_sender
                .send(command)
                .expect("Artnet sender closed its channel");
        }
    }
}

#[macro_export]
macro_rules! get_pipeline {
    ($field: ident) => {
        let wgpu_render_state = $crate::wgpu_render_state();
        let mut renderer = wgpu_render_state.renderer.write();
        let $field = renderer
            .paint_callback_resources
            .get_mut::<$crate::pipeline::Pipeline>()
            .expect("Could not find Pipeline");
    };
}
