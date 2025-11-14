pub mod scene_instance_path;

use super::{
    AssetTrait, animation::Animation, output_device::routing::OutputRoutings,
    scene::instance::SceneInstance,
};
use crate::{
    app::{svg::Svg, timing::Timing},
    input::{
        artnet::ArtnetConfig,
        event::{GamepadEvent, InputEvent},
    },
    pipeline::{
        extract_output::ExtractOutput,
        group::Groups,
        output_clear::OutputClear,
        preview::Preview,
        preview_indices::PreviewIndices,
        renderer_callback::RendererCallback,
        transition::{Transition, TransitionGoal},
    },
    storage::{
        asset::{
            Asset, palette::Palette, project::scene_instance_path::SceneInstancePathIndex,
            scene::Scene,
        },
        asset_id::AssetId,
    },
    ui::windows::channel_overwrites::ChannelOverwrites,
    wgpu_render_state,
};
use rand::seq::IndexedMutRandom;
use scene_instance_path::SceneInstancePathId;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashSet},
    time::{Duration, Instant},
};
use uuid::Uuid;
use wgpu::CommandEncoderDescriptor;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Project {
    pub palette: Option<AssetId<Palette>>,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,
    pub groups: Groups,
    pub scenes_instances_grid: Vec<SceneInstance>,
    pub scenes_instances_quick: Vec<SceneInstance>,

    #[serde(skip)]
    pub auto_mode_last_change: Option<Instant>,
    pub svg: Option<Svg>,
    pub channel_overwrites: ChannelOverwrites,
    pub output_routings: OutputRoutings,
    pub artnet_config: ArtnetConfig,
    pub tap_input_events: BTreeSet<InputEvent>,
    pub blackout_input_events: BTreeSet<InputEvent>,
    pub blackout_hold_input_events: BTreeSet<InputEvent>,
    pub half_input_events: BTreeSet<InputEvent>,
    pub double_input_events: BTreeSet<InputEvent>,
    pub main_dimmer: f32,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            palette: None,
            auto_mode_active: false,
            auto_mode_seconds: 10,
            auto_mode_max_scenes: 2,
            groups: Groups::default(),
            scenes_instances_grid: Vec::new(),
            scenes_instances_quick: Vec::new(),
            auto_mode_last_change: None,
            svg: Default::default(),
            channel_overwrites: Default::default(),
            output_routings: Default::default(),
            artnet_config: Default::default(),
            tap_input_events: std::iter::once(InputEvent::Key(egui::Key::T))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Mode(0))))
                .collect(),
            blackout_input_events: std::iter::once(InputEvent::Key(egui::Key::B)).collect(),
            blackout_hold_input_events: std::iter::once(InputEvent::Key(egui::Key::N))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Start(0))))
                .collect(),
            half_input_events: std::iter::once(InputEvent::Key(egui::Key::Minus)).collect(),
            double_input_events: std::iter::once(InputEvent::Key(egui::Key::Plus)).collect(),
            main_dimmer: 1.0,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckPath {
    #[default]
    Grid,
    Quick,
}

impl Project {
    #[inline(always)]
    pub fn all_scene_instances(&mut self) -> impl Iterator<Item = &mut SceneInstance> {
        self.scenes_instances_grid
            .iter_mut()
            .chain(self.scenes_instances_quick.iter_mut())
    }

    pub fn all_scene_instances_id(
        &mut self,
    ) -> impl Iterator<Item = (SceneInstancePathId, &mut SceneInstance)> {
        self.scenes_instances_grid
            .iter_mut()
            .map(|scene_instance| {
                (
                    SceneInstancePathId {
                        deck_path: DeckPath::Grid,
                        id: scene_instance.id,
                    },
                    scene_instance,
                )
            })
            .chain(
                self.scenes_instances_quick
                    .iter_mut()
                    .map(|scene_instance| {
                        (
                            SceneInstancePathId {
                                deck_path: DeckPath::Quick,
                                id: scene_instance.id,
                            },
                            scene_instance,
                        )
                    }),
            )
    }

    pub fn all_scene_instances_index(
        &mut self,
    ) -> impl Iterator<Item = (SceneInstancePathIndex, &mut SceneInstance)> {
        self.scenes_instances_grid
            .iter_mut()
            .enumerate()
            .map(|(index, scene_instance)| {
                (
                    SceneInstancePathIndex {
                        deck_path: DeckPath::Grid,
                        index,
                    },
                    scene_instance,
                )
            })
            .chain(self.scenes_instances_quick.iter_mut().enumerate().map(
                |(index, scene_instance)| {
                    (
                        SceneInstancePathIndex {
                            deck_path: DeckPath::Quick,
                            index,
                        },
                        scene_instance,
                    )
                },
            ))
    }

    pub fn reload_shader_code(&mut self, animation: AssetId<Animation>) {
        self.all_scene_instances().for_each(|scene_instance| {
            scene_instance.reload_shader_code(animation);
        });
    }

    pub fn send_positions(&mut self) {
        self.all_scene_instances().for_each(|scene_instance| {
            scene_instance.send_positions();
        });
    }

    pub fn init_gpu(&mut self) {
        self.all_scene_instances().for_each(|scene_instance| {
            scene_instance.init_states();
        });
        self.set_buffers();
    }

    pub fn set_buffers(&mut self) {
        self.all_scene_instances().for_each(|scene_instance| {
            scene_instance.set_output_mix_buffers();
        });
        Preview::set_buffers();
    }

    pub fn scene_instance(&self, path: SceneInstancePathId) -> Option<&SceneInstance> {
        match path.deck_path {
            DeckPath::Grid => self
                .scenes_instances_grid
                .iter()
                .find(|scene_instance| scene_instance.id == path.id),
            DeckPath::Quick => self
                .scenes_instances_quick
                .iter()
                .find(|scene_instance| scene_instance.id == path.id),
        }
    }

    #[inline(always)]
    pub fn scene_instance_mut(&mut self, path: SceneInstancePathId) -> Option<&mut SceneInstance> {
        match path.deck_path {
            DeckPath::Grid => self
                .scenes_instances_grid
                .iter_mut()
                .find(|scene_instance| scene_instance.id == path.id),
            DeckPath::Quick => self
                .scenes_instances_quick
                .iter_mut()
                .find(|scene_instance| scene_instance.id == path.id),
        }
    }

    pub fn scene_instance_by_index(
        &mut self,
        path: SceneInstancePathIndex,
    ) -> Option<&mut SceneInstance> {
        match path.deck_path {
            DeckPath::Grid => self.scenes_instances_grid.get_mut(path.index),
            DeckPath::Quick => self.scenes_instances_quick.get_mut(path.index),
        }
    }

    #[inline(always)]
    pub fn scene_instances(&mut self, deck_path: DeckPath) -> &mut Vec<SceneInstance> {
        match deck_path {
            DeckPath::Grid => &mut self.scenes_instances_grid,
            DeckPath::Quick => &mut self.scenes_instances_quick,
        }
    }

    /// Remove scene instance at path and update path to the next scene instance
    pub fn remove_scene_instance(
        &mut self,
        path: &mut SceneInstancePathId,
    ) -> Option<SceneInstance> {
        let scene_instances = match path.deck_path {
            DeckPath::Grid => &mut self.scenes_instances_grid,
            DeckPath::Quick => &mut self.scenes_instances_quick,
        };

        let pos = scene_instances.iter().position(|s| s.id == path.id)?;
        let scene_instance = scene_instances.remove(pos);
        path.id = scene_instances
            .get(pos.saturating_sub(1))
            .map_or_else(Uuid::nil, |s| s.id);
        Some(scene_instance)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        timing: &Timing,
        blackout: bool,
        always_render: bool,
        fade_duration: Duration,
    ) {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;
        let queue = &wgpu_render_state.queue;

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
                        .scenes_instances_grid
                        .iter()
                        .enumerate()
                        .filter(|(_index, scene)| scene.active)
                        .map(|(index, _scene)| index)
                        .collect::<Vec<_>>();

                    let mut disable_count =
                        (indices.len() + 1).saturating_sub(auto_mode_max_scenes);
                    while disable_count > 0 {
                        if let Some(index) = indices.choose_mut(&mut rand::rng()).copied()
                            && prev.insert(index)
                        {
                            disable_count -= 1;
                            if let Some(scene) = self.scenes_instances_grid.get_mut(index) {
                                scene.set_transition(Transition::new(
                                    TransitionGoal::TurnOff,
                                    fade_duration,
                                ));
                            }
                        }
                    }
                }

                let mut scenes = self
                    .scenes_instances_grid
                    .iter_mut()
                    .enumerate()
                    .filter(|(index, _scene)| !prev.contains(index))
                    .collect::<Vec<_>>();
                if let Some((_index, scene)) = scenes.choose_mut(&mut rand::rng()) {
                    scene.set_transition(Transition::new(TransitionGoal::TurnOn, fade_duration));
                }

                self.auto_mode_last_change.take();
            }
        } else {
            self.auto_mode_last_change.take();
        }

        let palette = self.palette.and_then(Asset::get);
        let deck_groups = self.groups.clone();
        let main_dimmer = self.main_dimmer;
        for scene_instance in self.all_scene_instances() {
            scene_instance.prepare(
                queue,
                always_render,
                palette.clone(),
                &deck_groups,
                timing,
                main_dimmer,
            );
        }

        PreviewIndices::get().prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        OutputClear::get().run(&mut encoder);

        for scene_instance in self.all_scene_instances() {
            scene_instance.render(&mut encoder, blackout, always_render);
        }

        ExtractOutput::get().run(&mut encoder);
        PreviewIndices::get().run(&mut encoder);
        Preview::run(&mut encoder);

        RendererCallback::add(encoder.finish());
    }

    pub fn tap_input_is_new(&self) -> bool {
        self.tap_input_events.iter().any(|event| event.is_new())
    }

    pub fn blackout_input_is_new(&self) -> bool {
        self.blackout_input_events
            .iter()
            .any(|event| event.is_new())
    }

    pub fn blackout_hold_input_is_live(&self) -> bool {
        self.blackout_hold_input_events
            .iter()
            .any(|event| event.is_live())
    }

    pub fn half_input_is_new(&self) -> bool {
        self.half_input_events.iter().any(|event| event.is_new())
    }

    pub fn double_input_is_new(&self) -> bool {
        self.double_input_events.iter().any(|event| event.is_new())
    }

    pub fn add_scene(&mut self, deck_path: DeckPath, scene: AssetId<Scene>) -> SceneInstancePathId {
        let mut scene_instance: SceneInstance = scene.into();
        scene_instance.init_states();
        self.add_scene_instance(deck_path, scene_instance)
    }

    pub fn add_scene_instance(
        &mut self,
        deck_path: DeckPath,
        scene_instance: SceneInstance,
    ) -> SceneInstancePathId {
        let scene_instances = match deck_path {
            DeckPath::Grid => &mut self.scenes_instances_grid,
            DeckPath::Quick => &mut self.scenes_instances_quick,
        };
        let path = SceneInstancePathId {
            deck_path,
            id: scene_instance.id,
        };
        scene_instances.push(scene_instance);
        path
    }

    pub fn remove_nonexistant_groups(&mut self) {
        self.groups.remove_nonexistant_groups();
        for scene_instance in self.all_scene_instances() {
            scene_instance.remove_nonexistant_groups();
        }
    }
}

impl AssetTrait for Project {
    const DIR_NAME: &'static str = "projects";
    const NAME: &'static str = "Project";
    const SHOW_NAME_IF_SELECTED: bool = true;
}
