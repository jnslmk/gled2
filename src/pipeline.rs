use crate::{
    app::Timing,
    constants::{GPU_NOT_INIT, OUTPUT_BUFFER_SIZE},
    extract_output::ExtractOutput,
    output_clear::OutputClear,
    output_sender::{GpuReadyReceiver, OutputSender},
    preview::Preview,
    preview_indices::PreviewIndices,
    scene_instance::SceneInstance,
    storage::{Asset, AssetId, Palette, Scene},
    svg::Universes,
    transition::{Transition, TransitionGoal},
    wgpu_render_state,
};
use egui::TextureId;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    iter::once,
    time::{Duration, Instant},
};
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Pipeline {
    pub scenes_instances: Vec<SceneInstance>,
    pub palette: Option<AssetId<Palette>>,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,

    #[serde(skip)]
    auto_mode_last_change: Option<Instant>,
    #[serde(skip)]
    start: Option<Instant>,
    #[serde(skip)]
    preview_indices: Option<PreviewIndices>,
    #[serde(skip)]
    preview: Option<Preview>,
    #[serde(skip)]
    output: Option<Buffer>,
    #[serde(skip)]
    output_clear: Option<OutputClear>,
}

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        Self {
            scenes_instances: self.scenes_instances.clone(),
            palette: self.palette,
            auto_mode_active: self.auto_mode_active,
            auto_mode_seconds: self.auto_mode_seconds,
            auto_mode_max_scenes: self.auto_mode_max_scenes,
            auto_mode_last_change: self.auto_mode_last_change,
            start: self.start,
            ..Default::default()
        }
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            scenes_instances: Default::default(),
            palette: Default::default(),
            auto_mode_last_change: Default::default(),
            auto_mode_active: false,
            auto_mode_seconds: 45,
            auto_mode_max_scenes: 3,
            start: Default::default(),
            preview_indices: Default::default(),
            preview: Default::default(),
            output: Default::default(),
            output_clear: Default::default(),
        }
    }
}

impl Pipeline {
    pub fn init_gpu(&mut self) {
        self.output_clear.get_or_insert_with(OutputClear::init);
        self.output.get_or_insert_with(|| {
            wgpu_render_state().device.create_buffer(&BufferDescriptor {
                size: OUTPUT_BUFFER_SIZE,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
                label: Some("Output buffer"),
                mapped_at_creation: false,
            })
        });
        self.preview_indices.get_or_insert_with(PreviewIndices::new);
        self.preview.get_or_insert_with(Preview::new);
        for scene_instance in self.scenes_instances.iter_mut() {
            scene_instance.init_states();
        }
        self.set_buffers();
    }

    pub fn add_scene(&mut self, scene: AssetId<Scene>) -> usize {
        let mut scene_instance: SceneInstance = scene.into();
        scene_instance.init_states();
        self.scenes_instances.push(scene_instance);

        self.scenes_instances.len() - 1
    }

    pub fn remove_scene_instance(&mut self, index: usize) {
        if self.scenes_instances.len() > index {
            self.scenes_instances.remove(index);
            self.init_gpu();
        }
    }

    pub fn set_buffers(&mut self) {
        let output = self.output.as_ref().expect(GPU_NOT_INIT);

        for scene in self.scenes_instances.iter_mut() {
            scene.set_buffers(output);
        }

        self.preview.as_mut().expect(GPU_NOT_INIT).set_buffers(
            self.preview_indices.as_ref().expect(GPU_NOT_INIT).indices(),
            output,
        );

        self.output_clear
            .as_mut()
            .expect(GPU_NOT_INIT)
            .set_buffers(output)
    }

    pub fn scene_instances(&mut self) -> Vec<(usize, &mut SceneInstance)> {
        self.scenes_instances.iter_mut().enumerate().collect()
    }

    pub fn scene_instance(&mut self, index: usize) -> Option<&mut SceneInstance> {
        self.scenes_instances.get_mut(index)
    }

    pub fn svg_or_groups_changed(&mut self, universes: Universes) {
        for scene_instance in self.scenes_instances.iter_mut() {
            scene_instance.send_positions();
        }
        self.preview_indices
            .as_mut()
            .expect(GPU_NOT_INIT)
            .send_positions();
        *ExtractOutput::get().universes.lock() = universes;
    }

    pub fn preview_texture_id(&self) -> TextureId {
        self.preview.as_ref().expect(GPU_NOT_INIT).texture_id()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        output_sender: &mut OutputSender,
        gpu_ready_receiver: &mut GpuReadyReceiver,
        timing: &Timing,
        blackout: bool,
        render_deactivated_scenes: RenderDeactivatedScenes,
        fade_duration: Duration,
    ) {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;
        let queue = &wgpu_render_state.queue;

        for scene_instance in self.scenes_instances.iter_mut() {
            scene_instance.set_state_timing(timing);
        }

        if self.auto_mode_active {
            if self
                .auto_mode_last_change
                .get_or_insert_with(Instant::now)
                .elapsed()
                .as_secs()
                > self.auto_mode_seconds
            {
                let auto_mode_max_scenes = self.auto_mode_max_scenes;
                let mut prev = HashSet::new();
                {
                    let mut indices = self
                        .scene_instances()
                        .into_iter()
                        .filter(|(_index, scene)| scene.active)
                        .map(|(index, _scene)| index)
                        .collect::<Vec<_>>();

                    let mut disable_count =
                        (indices.len() + 1).saturating_sub(auto_mode_max_scenes);
                    while disable_count > 0 {
                        if let Some(index) = indices.choose_mut(&mut rand::thread_rng()).copied() {
                            if prev.insert(index) {
                                disable_count -= 1;
                                if let Some(scene) = self.scene_instance(index) {
                                    scene.set_transition(Transition::new(
                                        TransitionGoal::TurnOff,
                                        fade_duration,
                                    ));
                                }
                            }
                        }
                    }
                }

                let mut scenes = self
                    .scene_instances()
                    .into_iter()
                    .filter(|(index, _scene)| !prev.contains(index))
                    .collect::<Vec<_>>();
                if let Some((_index, scene)) = scenes.choose_mut(&mut rand::thread_rng()) {
                    scene.set_transition(Transition::new(TransitionGoal::TurnOn, fade_duration));
                }

                self.auto_mode_last_change.take();
            }
        } else {
            self.auto_mode_last_change.take();
        }

        let palette = self.palette.and_then(Asset::get);
        for (index, scene_instance) in self.scene_instances() {
            scene_instance.prepare(
                queue,
                render_deactivated_scenes.should_render(index),
                palette.clone(),
            );
        }
        self.preview_indices
            .as_mut()
            .expect(GPU_NOT_INIT)
            .prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        self.output_clear
            .as_ref()
            .expect(GPU_NOT_INIT)
            .run(&mut encoder);

        for (index, scene) in self.scene_instances() {
            scene.render(
                &mut encoder,
                blackout,
                render_deactivated_scenes.should_render(index),
            );
        }

        ExtractOutput::get().run(&mut encoder, self.output.as_ref().expect(GPU_NOT_INIT));
        self.preview_indices
            .as_mut()
            .expect(GPU_NOT_INIT)
            .run(&mut encoder);
        self.preview.as_mut().expect(GPU_NOT_INIT).run(&mut encoder);

        //wait for gpu to be ready for the next queue submission
        gpu_ready_receiver.recv().ok();
        queue.submit(once(encoder.finish()));

        output_sender
            .send(())
            .expect("Output sender closed its channel");
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
