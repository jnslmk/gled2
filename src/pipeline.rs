use crate::{
    animation::State,
    artnet_clear::ArtnetClear,
    artnet_mix::ArtnetMix,
    artnet_sender::{ArtnetSender, GpuReadyReceiver},
    constants::ARTNET_BUFFER_SIZE,
    extract_artnet::ExtractArtnet,
    preview::Preview,
    preview_indices::PreviewIndices,
    scene::Scene,
    svg::Universes,
    wgpu_render_state,
};
use egui::TextureId;
use serde::{Deserialize, Serialize};
use slab::Slab;
use std::time::Instant;
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pipeline {
    #[serde(skip)]
    start: Option<Instant>,
    scenes: Slab<Scene>,
    #[serde(skip)]
    mixs: Vec<ArtnetMix>,
    #[serde(skip)]
    extract: Option<ExtractArtnet>,
    #[serde(skip)]
    preview_indices: Option<PreviewIndices>,
    #[serde(skip)]
    preview: Option<Preview>,
    #[serde(skip)]
    artnet: Option<Buffer>,
    #[serde(skip)]
    artnet_clear: Option<ArtnetClear>,
}

impl Pipeline {
    pub fn set_extract_artnet(&mut self, extract_artnet: ExtractArtnet) {
        self.extract = Some(extract_artnet);
    }

    pub fn init_gpu(&mut self) {
        self.artnet_clear.get_or_insert_with(ArtnetClear::init);
        self.artnet.get_or_insert_with(|| {
            wgpu_render_state().device.create_buffer(&BufferDescriptor {
                size: ARTNET_BUFFER_SIZE,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
                label: Some("Artnet buffer"),
                mapped_at_creation: false,
            })
        });
        self.preview_indices.get_or_insert_with(PreviewIndices::new);
        self.preview.get_or_insert_with(Preview::new);
        for (_index, scene) in self.scenes.iter_mut() {
            scene.init_gpu();
        }
        self.update_buffers();
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
        self.init_gpu();
    }

    pub fn update_buffers(&mut self) {
        let mut mix_index = 0;
        for (_index, scene) in self.scenes.iter_mut() {
            if self.mixs.len() <= mix_index {
                self.mixs.push(ArtnetMix::init());
            }
            self.mixs
                .get_mut(mix_index)
                .expect("Mix does not exist")
                .set_buffers(
                    self.artnet.as_ref().expect("Gpu was not yet initialized"),
                    scene.artnet_buffer(),
                );
            mix_index += 1;
        }

        self.mixs.shrink_to(mix_index);

        self.preview
            .as_mut()
            .expect("Gpu was not yet initialized")
            .set_buffers(
                self.preview_indices
                    .as_ref()
                    .expect("Gpu was not yet initialized")
                    .indices(),
                self.artnet.as_ref().expect("Gpu was not yet initialized"),
            );

        self.artnet_clear
            .as_mut()
            .expect("Gpu was not yet initialized")
            .set_buffers(self.artnet.as_ref().expect("Gpu was not yet initialized"))
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

    pub fn scenes(&mut self) -> Vec<(usize, &mut Scene)> {
        self.scenes.iter_mut().collect()
    }

    pub fn svg_or_groups_changed(&mut self, universes: Universes) {
        for (_index, scene) in self.scenes.iter_mut() {
            scene.send_positions();
        }
        self.preview_indices
            .as_mut()
            .expect("Gpu was not yet initialized")
            .send_positions();
        self.extract
            .as_mut()
            .expect("Gpu was not yet initialized")
            .set_universes(universes);
    }

    pub fn preview_texture_id(&self) -> TextureId {
        self.preview
            .as_ref()
            .expect("Gpu was not yet initialized")
            .texture_id()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        artnet_sender: &mut ArtnetSender,
        gpu_ready_receiver: &mut GpuReadyReceiver,
        beat_progression: f32,
        beats_per_minute: f32,
        framerate: f32,
        disable_artnet_extraction: bool,
        main_dimmer: f32,
    ) {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;
        let queue = &wgpu_render_state.queue;

        let time = self.start().elapsed().as_secs_f32();
        let state = State {
            time,
            beat_progression,
            beats_per_minute,
            framerate,
            ..Default::default()
        };

        for (_index, scene) in self.scenes.iter_mut() {
            scene.prepare(queue, state, disable_artnet_extraction, main_dimmer);
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

        self.artnet_clear
            .as_ref()
            .expect("Gpu is not yet initialized")
            .run(&mut encoder);
        for mix in self.mixs.iter() {
            mix.run(&mut encoder);
        }

        self.extract
            .as_mut()
            .expect("Gpu was not yet initialized")
            .run(
                &mut encoder,
                self.artnet.as_ref().expect("Gpu was not yet initialized"),
            );
        self.preview_indices
            .as_mut()
            .expect("Gpu was not yet initialized")
            .run(&mut encoder);
        self.preview
            .as_mut()
            .expect("Gpu was not yet initialized")
            .run(&mut encoder);

        //wait for gpu to be ready for the next queue submission
        gpu_ready_receiver.recv().ok();
        queue.submit(std::iter::once(encoder.finish()));

        artnet_sender
            .send(())
            .expect("Artnet sender closed its channel");
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
