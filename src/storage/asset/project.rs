pub mod scene_instance_path;

use super::{
    AssetTrait, animation::Animation, output_device::routing::OutputRoutings,
    scene::instance::SceneInstance,
};
use crate::{
    app::{svg::Svg, timing::Timing},
    audio::sound_data::SoundData,
    input::{
        artnet::{ARTNET_CONFIG, ArtnetConfig},
        event::{GamepadEvent, InputEvent},
        external_control::ArtnetControlConfig,
    },
    midi::akai_apc40_mk2::{GRID_HEIGHT, GRID_WIDTH},
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
            palette::Palette,
            project::scene_instance_path::SceneInstanceUnion,
            scene::{Scene, grid::GridLocation},
        },
        asset_id::AssetId,
        collections::Collections,
    },
    ui::windows::channel_overwrites::ChannelOverwrites,
    wgpu_render_state,
};
use cpal::DeviceId;
use rand::prelude::IndexedMutRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::CommandEncoderDescriptor;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Project {
    pub palette: Option<Palette>,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,
    pub groups: Groups,
    pub scenes_instances_grid: HashMap<GridLocation, SceneInstance>,
    #[serde(skip)]
    pub auto_mode_last_change: Option<Instant>,
    pub svg: Option<Svg>,
    pub channel_overwrites: ChannelOverwrites,
    pub output_routings: Arc<OutputRoutings>,
    pub artnet_config: ArtnetConfig,
    pub tap_input_events: BTreeSet<InputEvent>,
    pub blackout_input_events: BTreeSet<InputEvent>,
    pub blackout_hold_input_events: BTreeSet<InputEvent>,
    pub half_input_events: BTreeSet<InputEvent>,
    pub double_input_events: BTreeSet<InputEvent>,
    pub main_dimmer: f32,
    #[serde(
        serialize_with = "crate::audio::device_id_serde::serialize_device_id",
        deserialize_with = "crate::audio::device_id_serde::deserialize_scene_instances"
    )]
    pub audio_input_device: Option<DeviceId>,
    artnet_control_config: ArtnetControlConfig,
}

impl Default for Project {
    fn default() -> Self {
        #[cfg(feature = "profiling")]
        puffin::profile_function!("Project::default");
        Self {
            palette: None,
            auto_mode_active: false,
            auto_mode_seconds: 10,
            auto_mode_max_scenes: 2,
            groups: Groups::default(),
            scenes_instances_grid: HashMap::new(),
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
            audio_input_device: None,
            artnet_control_config: ArtnetControlConfig::default(),
        }
    }
}

impl Project {
    pub fn next_empty_grid_location(&self, start: GridLocation) -> GridLocation {
        for row in 0..GRID_HEIGHT {
            let row = (start.row + row) % GRID_HEIGHT;
            for col in 0..GRID_WIDTH {
                let col = (start.col + col) % GRID_WIDTH;
                let location = GridLocation { row, col };
                if !self.scenes_instances_grid.contains_key(&location) {
                    return location;
                }
            }
        }
        start
    }

    pub fn get_scenes_instance(&mut self, pos: &GridLocation) -> Option<&mut SceneInstance> {
        self.scenes_instances_grid.get_mut(pos)
    }
    pub fn scenes_instances_grid_len(&self) -> usize {
        self.scenes_instances_grid.len()
    }

    pub fn scene_instance_by_location_or_quick_index(
        &mut self,
        index_or_grid: SceneInstanceUnion,
    ) -> Option<&mut SceneInstance> {
        let pos = &self.location_by_location_or_quick_index(index_or_grid)?;
        self.get_scenes_instance(pos)
    }
    pub fn location_by_location_or_quick_index(
        &mut self,
        index_or_grid: SceneInstanceUnion,
    ) -> Option<GridLocation> {
        match index_or_grid {
            SceneInstanceUnion::Grid(location) => Some(location),
            SceneInstanceUnion::Quick(quick_scene_instance_index) => Some(GridLocation {
                row: GRID_HEIGHT - 1,
                col: quick_scene_instance_index.index,
            }),
        }
    }

    pub fn all_scene_instance_locations(&self) -> impl Iterator<Item = &GridLocation> {
        self.scenes_instances_grid.keys()
    }

    pub fn scenes_instances_quick(&self) -> impl Iterator<Item = (&GridLocation, &SceneInstance)> {
        self.scenes_instances_grid
            .iter()
            .filter(|(location, _)| location.row == GRID_HEIGHT - 1)
    }

    /// Reload shader code for all effects using the given animation, should be called after an animation is edited
    /// If the given animation is None, reloads all effects
    pub fn reload_shader_code(
        &mut self,
        animation: Option<AssetId<Animation>>,
        collections: &Collections,
    ) {
        self.scenes_instances_grid
            .values_mut()
            .for_each(|scene_instance| {
                scene_instance.reload_shader_code(animation, collections);
            });
    }

    pub fn send_positions(&mut self) {
        self.scenes_instances_grid
            .values_mut()
            .for_each(|scene_instance| {
                scene_instance.send_positions();
            });
    }

    /// Remove scene instance at path and update path to the next scene instance
    pub fn remove_scene_instance(&mut self, pos: GridLocation) -> Option<SceneInstance> {
        self.scenes_instances_grid.remove(&pos)
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn render(
        &mut self,
        timing: &Timing,
        blackout: bool,
        always_render: bool,
        fade_duration: Duration,
        collections: &Collections,
        extract_output: &ExtractOutput,
        sound_data: &SoundData,
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
                        .filter(|(_index, scene)| scene.active)
                        .map(|(location, _scene)| *location)
                        .collect::<Vec<_>>();

                    let mut disable_count =
                        (indices.len() + 1).saturating_sub(auto_mode_max_scenes);
                    while disable_count > 0 {
                        if let Some(location) = indices.choose_mut(&mut rand::rng()).copied()
                            && prev.insert(location)
                        {
                            disable_count -= 1;
                            if let Some(scene) = self.scenes_instances_grid.get_mut(&location) {
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
                    .filter(|(location, _scene)| !prev.contains(location))
                    .collect::<Vec<_>>();
                if let Some((_index, scene)) = scenes.choose_mut(&mut rand::rng()) {
                    scene.set_transition(Transition::new(TransitionGoal::TurnOn, fade_duration));
                }

                self.auto_mode_last_change.take();
            }
        } else {
            self.auto_mode_last_change.take();
        }

        let palette = self.palette.clone();
        let deck_groups = self.groups.clone();
        let main_dimmer = self.main_dimmer;
        for scene_instance in self.scenes_instances_grid.values_mut() {
            scene_instance.prepare(
                queue,
                palette.clone(),
            always_render,
                &deck_groups,
                timing,
                main_dimmer,
                collections,
                sound_data,
            );
        }

        PreviewIndices::get().prepare(queue);

        #[allow(unused_mut)]
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        #[cfg(feature = "profiling")]
        {
            let mut wgpu_profiler = crate::WGPU_PROFILER.lock();
            OutputClear::get().run(&mut wgpu_profiler.scope("OutputClear", &mut encoder));
            for scene_instance in self.scenes_instances_grid.values_mut() {
                scene_instance.render(
                    &mut wgpu_profiler.scope(
                        format!("Render scene \"{}\"", scene_instance.name),
                        &mut encoder,
                    ),
                    blackout,
                    always_render,
                );
            }
            extract_output.run(&mut wgpu_profiler.scope("ExtractOutput", &mut encoder));
            PreviewIndices::get().run(&mut wgpu_profiler.scope("PreviewIndices", &mut encoder));
            Preview::run(&mut wgpu_profiler.scope("Preview", &mut encoder));
            wgpu_profiler.resolve_queries(&mut encoder);
        }

        #[cfg(not(feature = "profiling"))]
        {
            OutputClear::get().run(&mut encoder);
            for scene_instance in self.scenes_instances_grid.values_mut() {
                scene_instance.render(&mut encoder, blackout, always_render);
            }
            extract_output.run(&mut encoder);
            PreviewIndices::get().run(&mut encoder);
            Preview::run(&mut encoder);
        }

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

    pub fn add_scene(
        &mut self,
        pos: GridLocation,
        scene_id: AssetId<Scene>,
        collections: &Collections,
    ) {
        let scene_instance = SceneInstance::from_scene_id(scene_id, collections);
        self.add_scene_instance(pos, scene_instance);
    }

    pub fn add_scene_instance(&mut self, pos: GridLocation, scene_instance: SceneInstance) {
        let scene_instances = &mut self.scenes_instances_grid;
        scene_instances.insert(pos, scene_instance);
    }

    pub fn remove_nonexistant_groups(&mut self) {
        self.groups.remove_nonexistant_groups();
        for scene_instance in self.scenes_instances_grid.values_mut() {
            scene_instance.remove_nonexistant_groups();
        }
    }

    pub fn artnet_control_config(&mut self, apply: impl FnOnce(&mut ArtnetControlConfig)) {
        apply(&mut self.artnet_control_config);
        ARTNET_CONFIG.lock().artnet_control_config = self.artnet_control_config;
    }

    pub fn artnet_control_config_value(&self) -> ArtnetControlConfig {
        self.artnet_control_config
    }
}

impl AssetTrait for Project {
    const DIR_NAME: &'static str = "projects";
    const NAME: &'static str = "Project";
    const SHOW_NAME_IF_SELECTED: bool = true;
}

pub fn grid_location_from_continuous_index(index: usize, start: &GridLocation) -> GridLocation {
    let index = index + start.col + start.row * GRID_WIDTH;
    let row = index / GRID_HEIGHT;
    let col = index % GRID_WIDTH;
    GridLocation { row, col }
}
