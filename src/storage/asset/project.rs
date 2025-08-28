pub mod render_deactivated_scenes;
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
        group::{Group, Groups},
        output_clear::OutputClear,
        preview::Preview,
        preview_indices::PreviewIndices,
        renderer_callback::RendererCallback,
        transition::{Transition, TransitionGoal},
    },
    storage::{
        asset::{Asset, palette::Palette, scene::Scene},
        asset_id::AssetId,
    },
    ui::windows::channel_overwrites::ChannelOverwrites,
    wgpu_render_state,
};
use rand::seq::IndexedMutRandom;
use render_deactivated_scenes::RenderDeactivatedScenes;
use scene_instance_path::SceneInstancePath;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    time::{Duration, Instant},
};
use wgpu::CommandEncoderDescriptor;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Project {
    pub palette: Option<AssetId<Palette>>,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,
    #[serde(deserialize_with = "deserialize_groups")]
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
    pub fn all_scene_instances(
        &mut self,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        self.scenes_instances_grid
            .iter_mut()
            .enumerate()
            .map(|(scene_instance, si)| {
                (
                    SceneInstancePath {
                        deck_path: DeckPath::Grid,
                        scene_instance,
                    },
                    si,
                )
            })
            .chain(self.scenes_instances_quick.iter_mut().enumerate().map(
                |(scene_instance, si)| {
                    (
                        SceneInstancePath {
                            deck_path: DeckPath::Quick,
                            scene_instance,
                        },
                        si,
                    )
                },
            ))
    }

    pub fn reload_shader_code(&mut self, animation: AssetId<Animation>) {
        self.all_scene_instances()
            .for_each(|(_path, scene_instance)| {
                scene_instance.reload_shader_code(animation);
            });
    }

    pub fn send_positions(&mut self) {
        self.all_scene_instances()
            .for_each(|(_path, scene_instance)| {
                scene_instance.send_positions();
            });
    }

    pub fn init_gpu(&mut self) {
        self.all_scene_instances()
            .for_each(|(_path, scene_instance)| {
                scene_instance.init_states();
            });
        self.set_buffers();
    }

    pub fn set_buffers(&mut self) {
        self.all_scene_instances()
            .for_each(|(_path, scene_instance)| {
                scene_instance.set_output_mix_buffers();
            });
        Preview::set_buffers();
    }

    #[inline(always)]
    pub fn scene_instance(&mut self, path: SceneInstancePath) -> Option<&mut SceneInstance> {
        match path.deck_path {
            DeckPath::Grid => self.scenes_instances_grid.get_mut(path.scene_instance),
            DeckPath::Quick => self.scenes_instances_quick.get_mut(path.scene_instance),
        }
    }

    #[inline(always)]
    pub fn scene_instances(
        &mut self,
        path: SceneInstancePath,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        let scene_instances = match path.deck_path {
            DeckPath::Grid => &mut self.scenes_instances_grid,
            DeckPath::Quick => &mut self.scenes_instances_quick,
        };
        scene_instances
            .iter_mut()
            .enumerate()
            .map(move |(scene_instance, si)| {
                (
                    SceneInstancePath {
                        deck_path: path.deck_path,
                        scene_instance,
                    },
                    si,
                )
            })
    }

    /// Remove scene instance at path and update path to the next scene instance
    pub fn remove_scene_instance(&mut self, path: &mut SceneInstancePath) {
        let scene_instances = match path.deck_path {
            DeckPath::Grid => &mut self.scenes_instances_grid,
            DeckPath::Quick => &mut self.scenes_instances_quick,
        };
        if path.scene_instance < scene_instances.len() {
            scene_instances.remove(path.scene_instance);
        }

        while path.scene_instance > 0 {
            if self.scene_instance(*path).is_some() {
                break;
            }
            path.scene_instance -= 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        timing: &Timing,
        blackout: bool,
        render_deactivated_scenes: RenderDeactivatedScenes,
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
                        if let Some(index) = indices.choose_mut(&mut rand::rng()).copied() {
                            if prev.insert(index) {
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
        for (path, scene_instance) in self.all_scene_instances() {
            scene_instance.prepare(
                queue,
                render_deactivated_scenes.should_render(path),
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

        for (path, scene_instance) in self.all_scene_instances() {
            scene_instance.render(
                &mut encoder,
                blackout,
                render_deactivated_scenes.should_render(path),
            );
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

    pub fn add_scene(&mut self, path: &mut SceneInstancePath, scene: AssetId<Scene>) {
        let mut scene_instance: SceneInstance = scene.into();
        scene_instance.init_states();

        let scene_instances = match path.deck_path {
            DeckPath::Grid => &mut self.scenes_instances_grid,
            DeckPath::Quick => &mut self.scenes_instances_quick,
        };
        scene_instances.push(scene_instance);
        path.scene_instance = scene_instances.len() - 1;
    }

    pub fn remove_nonexistant_groups(&mut self) {
        self.groups.remove_nonexistant_groups();
        for (_path, scene_instance) in self.all_scene_instances() {
            scene_instance.remove_nonexistant_groups();
        }
    }
}

impl AssetTrait for Project {
    const DIR_NAME: &'static str = "projects";
    const NAME: &'static str = "Project";
    const SHOW_NAME_IF_SELECTED: bool = true;
}

fn deserialize_groups<'de, D>(deserializer: D) -> Result<Groups, D::Error>
where
    D: serde::Deserializer<'de>,
{
    BTreeMap::<String, Group>::deserialize(deserializer).map(|map| {
        Groups::new(
            map.into_iter()
                .filter_map(|(index, group)| index.parse().ok().map(|index| (index, group)))
                .collect(),
        )
    })
}
