use crate::{
    animation::{
        Color, ColorPalette, CommonConfig, Direction, Gradient, GradientType, State, Stripes,
    },
    artnet_clear::ArtnetClear,
    artnet_sender::{ArtnetSender, GpuReadyReceiver},
    constants::{ARTNET_BUFFER_SIZE, GPU_NOT_INIT},
    extract_artnet::ExtractArtnet,
    preview::Preview,
    preview_indices::PreviewIndices,
    scene::{Scene, SceneKind},
    svg::Universes,
    wgpu_render_state,
};
use egui::TextureId;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor};

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Pipeline {
    scenes: Vec<Scene>,
    #[serde(skip)]
    start: Option<Instant>,
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

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        Self {
            scenes: self.scenes.clone(),
            ..Default::default()
        }
    }
}

impl Pipeline {
    pub fn demo() -> Self {
        let mut pipeline = Self::default();

        for i in 0..10 {
            let palette = ColorPalette {
                colors: vec![Color::new(1., 0., 0.), Color::new(0., 0., 0.)],
            };
            let gradient = Gradient {
                gradient: GradientType::Radial,
                center: (0.25, 0.5),
                ..Default::default()
            };
            let mut scene = Scene::new(gradient.into(), palette, "allFull".to_owned());
            scene.active = i == 0;
            pipeline.add_scene(scene);

            let palette = ColorPalette {
                colors: vec![Color::new(0., 0., 1.), Color::new(0., 0., 0.)],
            };
            let gradient = Gradient {
                gradient: GradientType::LinearHorizontal,
                common: CommonConfig {
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut scene = Scene::new(gradient.into(), palette, "innerFull".to_owned());
            scene.active = i == 0;
            pipeline.add_scene(scene);

            let palette = ColorPalette {
                colors: vec![Color::new(1., 1., 0.)],
            };
            let stripes = Stripes {
                count: 2,
                common: CommonConfig {
                    direction: Direction::Backward,
                },
                ..Default::default()
            };
            let mut scene = Scene::new(stripes.into(), palette, "innerEdge".to_owned());
            scene.kind = SceneKind::Foreground;
            scene.active = i == 0;
            pipeline.add_scene(scene);
        }

        pipeline
    }

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
        for scene in self.scenes.iter_mut() {
            scene.init_gpu();
        }
        self.set_buffers();
    }

    pub fn start(&mut self) -> Instant {
        *self.start.get_or_insert_with(Instant::now)
    }

    pub fn add_scene(&mut self, scene: Scene) -> usize {
        self.scenes.push(scene);
        self.init_gpu();

        self.scenes.len() - 1
    }

    pub fn remove_scene(&mut self, index: usize) -> Option<Scene> {
        let mut scene = None;

        if self.scenes.len() > index {
            scene = Some(self.scenes.remove(index));
            self.init_gpu();
        }

        scene
    }

    pub fn set_buffers(&mut self) {
        for scene in self.scenes.iter_mut() {
            scene.set_buffers(self.artnet.as_ref().expect(GPU_NOT_INIT))
        }

        self.preview.as_mut().expect(GPU_NOT_INIT).set_buffers(
            self.preview_indices.as_ref().expect(GPU_NOT_INIT).indices(),
            self.artnet.as_ref().expect(GPU_NOT_INIT),
        );

        self.artnet_clear
            .as_mut()
            .expect(GPU_NOT_INIT)
            .set_buffers(self.artnet.as_ref().expect(GPU_NOT_INIT))
    }

    pub fn set_opacity(&mut self, index: usize, opacity: f32) {
        if let Some(scene) = self.scene(index) {
            scene.opacity = opacity;
        }
    }

    pub fn set_active(&mut self, index: usize, active: bool) {
        if let Some(scene) = self.scene(index) {
            scene.active = active;
        }
    }

    pub fn scenes(&mut self) -> Vec<(usize, &mut Scene)> {
        self.scenes.iter_mut().enumerate().collect()
    }

    pub fn scene(&mut self, index: usize) -> Option<&mut Scene> {
        self.scenes.get_mut(index)
    }

    pub fn svg_or_groups_changed(&mut self, universes: Universes) {
        for scene in self.scenes.iter_mut() {
            scene.send_positions();
        }
        self.preview_indices
            .as_mut()
            .expect(GPU_NOT_INIT)
            .send_positions();
        self.extract
            .as_mut()
            .expect(GPU_NOT_INIT)
            .set_universes(universes);
    }

    pub fn preview_texture_id(&self) -> TextureId {
        self.preview.as_ref().expect(GPU_NOT_INIT).texture_id()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        artnet_sender: &mut ArtnetSender,
        gpu_ready_receiver: &mut GpuReadyReceiver,
        beat_progression: f32,
        beats_per_minute: f32,
        framerate: f32,
        blackout: bool,
        main_dimmer: f32,
        render_deactivated_background_scenes: RenderDeactivatedScenes,
        render_deactivated_foreground_scenes: RenderDeactivatedScenes,
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

        for (index, scene) in self.scenes() {
            scene.prepare(
                queue,
                state,
                blackout,
                main_dimmer,
                match scene.kind {
                    crate::scene::SceneKind::Background => {
                        render_deactivated_background_scenes.should_render(index)
                    }
                    crate::scene::SceneKind::Foreground => {
                        render_deactivated_foreground_scenes.should_render(index)
                    }
                },
            );
        }
        self.preview_indices
            .as_mut()
            .expect(GPU_NOT_INIT)
            .prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        self.artnet_clear
            .as_ref()
            .expect(GPU_NOT_INIT)
            .run(&mut encoder);

        for (index, scene) in self.scenes() {
            scene.render(
                &mut encoder,
                blackout,
                match scene.kind {
                    crate::scene::SceneKind::Background => {
                        render_deactivated_background_scenes.should_render(index)
                    }
                    crate::scene::SceneKind::Foreground => {
                        render_deactivated_foreground_scenes.should_render(index)
                    }
                },
            );
        }

        self.extract
            .as_mut()
            .expect(GPU_NOT_INIT)
            .run(&mut encoder, self.artnet.as_ref().expect(GPU_NOT_INIT));
        self.preview_indices
            .as_mut()
            .expect(GPU_NOT_INIT)
            .run(&mut encoder);
        self.preview.as_mut().expect(GPU_NOT_INIT).run(&mut encoder);

        //wait for gpu to be ready for the next queue submission
        gpu_ready_receiver.recv().ok();
        queue.submit(std::iter::once(encoder.finish()));

        artnet_sender
            .send(())
            .expect("Artnet sender closed its channel");
    }
}

pub enum RenderDeactivatedScenes {
    Always,
    Some(usize, usize),
}

impl RenderDeactivatedScenes {
    pub fn should_render(&self, index: usize) -> bool {
        match self {
            RenderDeactivatedScenes::Always => true,
            RenderDeactivatedScenes::Some(selected, hovered) => {
                *selected == index || *hovered == index
            }
        }
    }
}
