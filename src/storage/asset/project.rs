use super::{Animation, Asset, AssetId, AssetTrait, Palette, Scene};
use crate::{
    app::{Svg, Timing},
    extract_output::ExtractOutput,
    group::Groups,
    output_clear::OUTPUT_CLEAR,
    output_routings::OutputRoutings,
    output_sender::{GpuReadyReceiver, OutputSender},
    preview::PREVIEW,
    preview_indices::PREVIEW_INDICES,
    scene_instance::SceneInstance,
    transition::{Transition, TransitionGoal},
    wgpu_render_state,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    iter::once,
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::{CommandEncoder, CommandEncoderDescriptor};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub a: Deck,
    pub b: Deck,
    /// -1.0 = A, 0.0 = A + B, 1.0 = B
    pub cross_fader: f32,
    pub svg: Option<Svg>,
    //TODO: set from extract output on save
    pub output_routings: OutputRoutings,
}

//TODO: Remove
impl Default for Project {
    fn default() -> Self {
        Self {
            a: Deck {
                scene_groups: vec![Default::default()],
                palette: None,
                auto_mode_active: Default::default(),
                auto_mode_seconds: Default::default(),
                auto_mode_max_scenes: Default::default(),
                auto_mode_last_change: Default::default(),
            },
            b: Default::default(),
            cross_fader: Default::default(),
            svg: Default::default(),
            output_routings: Default::default(),
        }
    }
}

impl Project {
    #[inline(always)]
    pub fn decks(&mut self) -> impl Iterator<Item = (SceneInstancePath, &mut Deck)> {
        once((
            SceneInstancePath {
                deck_a: true,
                ..Default::default()
            },
            &mut self.a,
        ))
        .chain(once((
            SceneInstancePath {
                deck_a: false,
                ..Default::default()
            },
            &mut self.b,
        )))
    }

    #[inline(always)]
    pub fn scene_instances(
        &mut self,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        self.decks().flat_map(|(path, deck)| {
            deck.scene_groups(path)
                .flat_map(|(path, scene_groups)| scene_groups.scene_instances(path))
        })
    }

    pub fn reload_shader_code(&mut self, animation: AssetId<Animation>) {
        self.scene_instances().for_each(|(_path, scene_instance)| {
            scene_instance.reload_shader_code(animation);
        });
    }

    pub fn send_positions(&mut self) {
        self.scene_instances().for_each(|(_path, scene_instance)| {
            scene_instance.send_positions();
        });
    }

    pub fn init_gpu(&mut self) {
        self.scene_instances().for_each(|(_path, scene_instance)| {
            scene_instance.init_states();
        });
        self.set_buffers();
    }

    pub fn set_buffers(&mut self) {
        self.scene_instances().for_each(|(_path, scene_instance)| {
            scene_instance.set_output_mix_buffers();
        });
        PREVIEW.lock().set_buffers();
    }

    #[inline(always)]
    pub fn deck(&mut self, path: SceneInstancePath) -> &mut Deck {
        if path.deck_a {
            &mut self.a
        } else {
            &mut self.b
        }
    }

    #[inline(always)]
    pub fn scene_group(&mut self, path: SceneInstancePath) -> Option<&mut SceneGroup> {
        self.deck(path).scene_group(path)
    }

    #[inline(always)]
    pub fn scene_instance(&mut self, path: SceneInstancePath) -> Option<&mut SceneInstance> {
        self.scene_group(path)?.scene_instance(path)
    }

    /// Remove scene instance at path and update path to the next scene instance
    pub fn remove_scene_instance(&mut self, path: &mut SceneInstancePath) {
        if let Some(scene_group) = self.scene_group(*path) {
            scene_group.scenes_instances.remove(path.scene_instance);
        }
        while path.scene_instance > 0 {
            if self.scene_instance(*path).is_some() {
                break;
            }
            path.scene_instance -= 1;
        }
        while path.scene_group > 0 {
            if self.scene_instance(*path).is_some() {
                break;
            }
            path.scene_group -= 1;
        }
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

        self.decks().for_each(|(path, deck)| {
            deck.prepare(
                path,
                queue,
                timing,
                render_deactivated_scenes,
                fade_duration,
            )
        });

        PREVIEW_INDICES.lock().prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        OUTPUT_CLEAR.lock().run(&mut encoder);

        self.decks().for_each(|(path, deck)| {
            deck.render(path, &mut encoder, blackout, render_deactivated_scenes)
        });

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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Deck {
    pub palette: Option<AssetId<Palette>>,
    pub scene_groups: Vec<SceneGroup>,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,

    #[serde(skip)]
    pub auto_mode_last_change: Option<Instant>,
}

impl Deck {
    pub fn scene_groups(
        &mut self,
        path: SceneInstancePath,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneGroup)> {
        self.scene_groups
            .iter_mut()
            .enumerate()
            .map(move |(scene_group, sg)| {
                (
                    SceneInstancePath {
                        scene_group,
                        ..path
                    },
                    sg,
                )
            })
    }

    #[inline(always)]
    pub fn scene_group(&mut self, path: SceneInstancePath) -> Option<&mut SceneGroup> {
        self.scene_groups.get_mut(path.scene_group)
    }

    #[inline(always)]
    pub fn scene_instances(
        &mut self,
        path: SceneInstancePath,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        self.scene_groups(path)
            .flat_map(|(path, scene_group)| scene_group.scene_instances(path))
    }

    #[inline(always)]
    pub fn scene_instance(&mut self, path: SceneInstancePath) -> Option<&mut SceneInstance> {
        self.scene_groups
            .get_mut(path.scene_group)?
            .scene_instance(path)
    }

    pub fn prepare(
        &mut self,
        path: SceneInstancePath,
        queue: &wgpu::Queue,
        timing: &Timing,
        render_deactivated_scenes: RenderDeactivatedScenes,
        fade_duration: Duration,
    ) {
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
                    let mut paths = self
                        .scene_instances(path)
                        .filter(|(_path, scene)| scene.active)
                        .map(|(path, _scene)| path)
                        .collect::<Vec<_>>();

                    let mut disable_count = (paths.len() + 1).saturating_sub(auto_mode_max_scenes);
                    while disable_count > 0 {
                        if let Some(path) = paths.choose_mut(&mut rand::thread_rng()).copied() {
                            if prev.insert(path) {
                                disable_count -= 1;
                                if let Some(scene) = self.scene_instance(path) {
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
                    .scene_instances(path)
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
        for (path, scene_group) in self.scene_groups(path) {
            scene_group.prepare(
                path,
                queue,
                timing,
                render_deactivated_scenes,
                palette.clone(),
            );
        }
    }

    pub fn render(
        &mut self,
        path: SceneInstancePath,
        encoder: &mut CommandEncoder,
        blackout: bool,
        render_deactivated_scenes: RenderDeactivatedScenes,
    ) {
        for (path, scene_group) in self.scene_groups(path) {
            scene_group.render(path, encoder, blackout, render_deactivated_scenes);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SceneGroup {
    pub groups: Groups,
    pub scenes_instances: Vec<SceneInstance>,
}

impl SceneGroup {
    pub fn add_scene(&mut self, path: &mut SceneInstancePath, scene: AssetId<Scene>) {
        let mut scene_instance: SceneInstance = scene.into();
        scene_instance.init_states();
        self.scenes_instances.push(scene_instance);

        path.scene_instance = self.scenes_instances.len() - 1;
    }

    #[inline(always)]
    pub fn scene_instances(
        &mut self,
        path: SceneInstancePath,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        self.scenes_instances
            .iter_mut()
            .enumerate()
            .map(move |(scene_instance, si)| {
                (
                    SceneInstancePath {
                        scene_instance,
                        ..path
                    },
                    si,
                )
            })
    }

    #[inline(always)]
    pub fn scene_instance(&mut self, path: SceneInstancePath) -> Option<&mut SceneInstance> {
        self.scenes_instances.get_mut(path.scene_instance)
    }

    pub fn prepare(
        &mut self,
        path: SceneInstancePath,
        queue: &wgpu::Queue,
        timing: &Timing,
        render_deactivated_scenes: RenderDeactivatedScenes,
        palette: Option<Arc<Asset<Palette>>>,
    ) {
        let groups = self.groups.clone();
        for (path, scene_instance) in self.scene_instances(path) {
            scene_instance.prepare(
                queue,
                render_deactivated_scenes.should_render(path),
                palette.clone(),
                &groups,
                timing,
            );
        }
    }

    pub fn render(
        &mut self,
        path: SceneInstancePath,
        encoder: &mut CommandEncoder,
        blackout: bool,
        render_deactivated_scenes: RenderDeactivatedScenes,
    ) {
        for (path, scene_instance) in self.scene_instances(path) {
            scene_instance.render(
                encoder,
                blackout,
                render_deactivated_scenes.should_render(path),
            );
        }
    }
}

impl AssetTrait for Project {
    const DIR_NAME: &'static str = "projects";
    const NAME: &'static str = "Project";
    const SHOW_NAME_IF_SELECTED: bool = true;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneInstancePath {
    pub deck_a: bool,
    pub scene_group: usize,
    pub scene_instance: usize,
}

impl Default for SceneInstancePath {
    fn default() -> Self {
        Self {
            deck_a: true,
            scene_group: 0,
            scene_instance: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderDeactivatedScenes {
    Always,
    Some(SceneInstancePath, SceneInstancePath),
}

impl RenderDeactivatedScenes {
    pub fn should_render(&self, path: SceneInstancePath) -> bool {
        match self {
            RenderDeactivatedScenes::Always => true,
            RenderDeactivatedScenes::Some(selected, hovered) => {
                *selected == path || *hovered == path
            }
        }
    }
}
