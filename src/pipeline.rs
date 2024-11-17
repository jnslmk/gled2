use crate::{
    app::Timing,
    constants::OUTPUT_BUFFER_SIZE,
    extract_output::ExtractOutput,
    group::Groups,
    output_clear::OUTPUT_CLEAR,
    output_sender::{GpuReadyReceiver, OutputSender},
    preview::PREVIEW,
    preview_indices::PREVIEW_INDICES,
    scene_instance::SceneInstance,
    storage::{Asset, AssetId, Palette, Scene},
    svg::Universes,
    transition::{Transition, TransitionGoal},
    wgpu_render_state,
};
use egui::{mutex::Mutex, TextureId};
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    iter::once,
    time::{Duration, Instant},
};
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor};

pub static AUTO_MODE_LAST_CHANGE: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));
pub static OUTPUT_BUFFER: Lazy<Buffer> = Lazy::new(|| {
    wgpu_render_state().device.create_buffer(&BufferDescriptor {
        size: OUTPUT_BUFFER_SIZE,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        label: Some("Output buffer"),
        mapped_at_creation: false,
    })
});

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Pipeline {
    pub scenes_instances: Vec<SceneInstance>,
    pub palette: Option<AssetId<Palette>>,
    pub groups: Groups,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            scenes_instances: Default::default(),
            palette: Default::default(),
            groups: Default::default(),
            auto_mode_active: false,
            auto_mode_seconds: 45,
            auto_mode_max_scenes: 3,
        }
    }
}

impl Pipeline {
    pub fn init_gpu(&mut self) {
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
        for scene in self.scenes_instances.iter_mut() {
            scene.set_buffers();
        }

        PREVIEW.lock().set_buffers(PREVIEW_INDICES.lock().indices());
        OUTPUT_CLEAR.lock().set_buffers();
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

        PREVIEW_INDICES.lock().send_positions();
        *ExtractOutput::get().universes.lock() = universes;
    }

    pub fn preview_texture_id(&self) -> TextureId {
        PREVIEW.lock().texture_id()
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

        if self.auto_mode_active {
            if AUTO_MODE_LAST_CHANGE
                .lock()
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

                AUTO_MODE_LAST_CHANGE.lock().take();
            }
        } else {
            AUTO_MODE_LAST_CHANGE.lock().take();
        }

        let palette = self.palette.and_then(Asset::get);
        let groups = self.groups.clone();
        for (index, scene_instance) in self.scene_instances() {
            scene_instance.prepare(
                queue,
                render_deactivated_scenes.should_render(index),
                palette.clone(),
                &groups,
                timing,
            );
        }
        PREVIEW_INDICES.lock().prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        OUTPUT_CLEAR.lock().run(&mut encoder);

        for (index, scene) in self.scene_instances() {
            scene.render(
                &mut encoder,
                blackout,
                render_deactivated_scenes.should_render(index),
            );
        }

        ExtractOutput::get().run(&mut encoder);
        PREVIEW_INDICES.lock().run(&mut encoder);
        PREVIEW.lock().run(&mut encoder);

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
