pub mod scene_instance_path;

use super::{
    AssetTrait, animation::Animation, midi_controller::MidiController,
    output_device::routing::OutputRoutings, scene::instance::SceneInstance,
};
use crate::{
    app::{svg::Svg, timing::Timing},
    audio::sound_data::SoundData,
    input::{
        artnet::{ARTNET_CONFIG, ArtnetConfig},
        event::{GamepadEvent, InputEvent},
        external_control::ArtnetControlConfig,
        osc::OscConfig,
    },
    pipeline::{
        extract_output::ExtractOutput, group::Groups, output_clear::OutputClear, preview::Preview,
        preview_indices::PreviewIndices,
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
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::CommandEncoderDescriptor;

/// How often a static frame is re-rendered and re-sent while no scene instance
/// counts as changed. 3 Hz keeps a rebooted endpoint (which boots black and
/// otherwise latches darkness) re-synced within ~333 ms with no operator
/// action, while skipping ~99% of the encode/submit/readback/send work. Stays
/// inside the required 2-4 Hz band; raise it (shorter interval) if endpoints
/// need tighter re-sync, lower it to save more CPU on long static holds.
const OUTPUT_KEEPALIVE_INTERVAL: Duration = Duration::from_millis(333);

/// Per-tick gate for the GPU output path. A tick runs the full
/// encode/submit/readback/send pipeline while changed, once on a falling edge,
/// and when the static keepalive is due. Feeding it an explicit `now` keeps
/// the decision deterministic and unit-testable without a GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OutputGate {
    last_render: Option<Instant>,
    was_changed: bool,
}

impl OutputGate {
    fn new() -> Self {
        Self {
            last_render: None,
            was_changed: false,
        }
    }

    /// Whether this tick must run the output pipeline. A falling edge renders
    /// once to flush deactivated or released output before static ticks resume
    /// skipping. Every render refreshes the keepalive clock, and the first tick
    /// after startup always renders because no last frame exists.
    fn should_render(&mut self, changed: bool, now: Instant) -> bool {
        let due = self
            .last_render
            .is_none_or(|last| now.saturating_duration_since(last) >= OUTPUT_KEEPALIVE_INTERVAL);
        let falling_edge = self.was_changed && !changed;
        self.was_changed = changed;
        if changed || falling_edge || due {
            self.last_render = Some(now);
            true
        } else {
            false
        }
    }
}

impl Default for OutputGate {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GridHighlight {
    Row,
    Column,
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Project {
    pub palette: Option<Palette>,
    pub groups: Groups,
    #[serde(default = "Project::default_grid_width")]
    pub grid_width: usize,
    #[serde(default = "Project::default_grid_height")]
    pub grid_height: usize,
    #[serde(default = "Project::default_grid_highlight")]
    pub grid_highlight: GridHighlight,
    pub scenes_instances_grid: HashMap<GridLocation, SceneInstance>,
    pub svg: Option<Svg>,
    pub channel_overwrites: ChannelOverwrites,
    pub output_routings: Arc<OutputRoutings>,
    pub artnet_config: ArtnetConfig,
    pub tap_input_events: BTreeSet<InputEvent>,
    pub blackout_input_events: BTreeSet<InputEvent>,
    pub blackout_hold_input_events: BTreeSet<InputEvent>,
    pub half_input_events: BTreeSet<InputEvent>,
    pub double_input_events: BTreeSet<InputEvent>,
    pub midi_active_mappings: HashMap<String, Option<AssetId<MidiController>>>,
    pub main_dimmer: f32,
    #[serde(
        serialize_with = "crate::audio::device_id_serde::serialize_device_id",
        deserialize_with = "crate::audio::device_id_serde::deserialize_scene_instances"
    )]
    pub audio_input_device: Option<DeviceId>,
    artnet_control_config: ArtnetControlConfig,
    pub osc_config: OscConfig,
    /// Runtime-only static-frame gate state. Skipped by serde (like
    /// `SceneInstance::flash`): never persisted, always fresh on load.
    #[serde(skip)]
    output_gate: OutputGate,
}

impl Default for Project {
    fn default() -> Self {
        #[cfg(feature = "profiling")]
        puffin::profile_function!("Project::default");
        Self {
            palette: None,
            groups: Groups::default(),
            grid_width: Self::DEFAULT_GRID_WIDTH,
            grid_height: Self::DEFAULT_GRID_HEIGHT,
            grid_highlight: Self::DEFAULT_GRID_HIGHLIGHT,
            scenes_instances_grid: HashMap::new(),
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
            midi_active_mappings: HashMap::new(),
            main_dimmer: 1.0,
            audio_input_device: None,
            artnet_control_config: ArtnetControlConfig::default(),
            osc_config: OscConfig::default(),
            output_gate: OutputGate::new(),
        }
    }
}

impl Project {
    pub const MIN_GRID_WIDTH: usize = 2;
    pub const MIN_GRID_HEIGHT: usize = 2;
    pub const DEFAULT_GRID_WIDTH: usize = 8;
    pub const DEFAULT_GRID_HEIGHT: usize = 6;
    pub const DEFAULT_GRID_HIGHLIGHT: GridHighlight = GridHighlight::Row;

    fn default_grid_width() -> usize {
        Self::DEFAULT_GRID_WIDTH
    }

    fn default_grid_height() -> usize {
        Self::DEFAULT_GRID_HEIGHT
    }

    fn default_grid_highlight() -> GridHighlight {
        Self::DEFAULT_GRID_HIGHLIGHT
    }

    pub fn grid_width(&self) -> usize {
        self.grid_width.max(Self::MIN_GRID_WIDTH)
    }

    pub fn grid_height(&self) -> usize {
        self.grid_height.max(Self::MIN_GRID_HEIGHT)
    }

    pub fn quick_row_index(&self) -> usize {
        self.grid_height() - 1
    }

    pub fn quick_col_index(&self) -> usize {
        self.grid_width() - 1
    }

    pub fn set_grid_size(&mut self, width: usize, height: usize) {
        self.grid_width = width.max(Self::MIN_GRID_WIDTH);
        self.grid_height = height.max(Self::MIN_GRID_HEIGHT);

        let max_width = self.grid_width();
        let max_height = self.grid_height();
        self.scenes_instances_grid
            .retain(|location, _| location.col < max_width && location.row < max_height);
    }

    pub fn next_empty_grid_location(&self, start: GridLocation) -> GridLocation {
        let grid_height = self.grid_height();
        let grid_width = self.grid_width();
        for row in 0..grid_height {
            let row = (start.row + row) % grid_height;
            for col in 0..grid_width {
                let col = (start.col + col) % grid_width;
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
            SceneInstanceUnion::Selected => None,
            SceneInstanceUnion::Grid(location) => Some(location),
            SceneInstanceUnion::Quick(quick_scene_instance_index) => match self.grid_highlight {
                GridHighlight::Row => {
                    if quick_scene_instance_index.index >= self.grid_width() {
                        return None;
                    }
                    Some(GridLocation {
                        row: self.quick_row_index(),
                        col: quick_scene_instance_index.index,
                    })
                }
                GridHighlight::Column => {
                    if quick_scene_instance_index.index >= self.grid_height() {
                        return None;
                    }
                    Some(GridLocation {
                        row: quick_scene_instance_index.index,
                        col: self.quick_col_index(),
                    })
                }
                GridHighlight::None => None,
            },
        }
    }

    pub fn all_scene_instance_locations(&self) -> impl Iterator<Item = &GridLocation> {
        self.scenes_instances_grid.keys()
    }

    pub fn scenes_instances_quick(&self) -> impl Iterator<Item = (&GridLocation, &SceneInstance)> {
        self.scenes_instances_grid
            .iter()
            .filter(move |(location, _)| match self.grid_highlight {
                GridHighlight::Row => location.row == self.quick_row_index(),
                GridHighlight::Column => location.col == self.quick_col_index(),
                GridHighlight::None => false,
            })
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
        collections: &Collections,
        extract_output: &mut ExtractOutput,
        sound_data: &SoundData,
    ) {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;
        let queue = &wgpu_render_state.queue;

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

        // Static-frame gate: when no scene instance counts as changed this tick,
        // skip the whole encode/submit/readback path below, so a static hold
        // costs no GPU work and sends no packets. `prepare` above still ran, so
        // activation and flash edges are observed at full rate. A falling edge
        // renders once to flush the inactive frame; otherwise the keepalive
        // re-renders and re-sends the unchanged output so rebooted endpoints
        // re-sync unaided.
        let changed = self.output_changed(always_render);
        if !self.output_gate.should_render(changed, Instant::now()) {
            return;
        }

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

        // Submit the animation/output work directly instead of deferring it to
        // egui's paint callback. This decouples the GPU->CPU readback (and the
        // resulting Art-Net/DMX output) from the surface-present path, so the
        // output for this frame is dispatched at the start of the frame and no
        // longer waits behind `Surface::get_current_texture` (which can stall
        // for whole vblank intervals when we render faster than the compositor
        // presents). The preview/output textures are written before egui samples
        // them later in the same frame, so ordering is preserved.
        let submission = queue.submit([encoder.finish()]);
        // Let the output poll thread deliver this frame's readback as soon as
        // the GPU finishes it, independently of any other frame submitted in the
        // same displayed frame (e.g. the second `double_render` pass).
        extract_output.notify_submitted(submission);
    }

    /// Whether any scene instance contributes output this tick. This is the
    /// exact condition under which `SceneInstance::render` does work (active,
    /// flash, or forced always-render) — kept in sync with its early return by
    /// the gate tests below, never redefined here.
    fn output_changed(&self, always_render: bool) -> bool {
        always_render
            || self
                .scenes_instances_grid
                .values()
                .any(|instance| instance.active || instance.flash)
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
        if pos.col >= self.grid_width() || pos.row >= self.grid_height() {
            return;
        }
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

    pub fn grid_location_from_continuous_index(
        &self,
        index: usize,
        start: &GridLocation,
    ) -> GridLocation {
        let grid_width = self.grid_width();
        let grid_height = self.grid_height();
        let span = grid_width * grid_height;
        let index = (index + start.col + start.row * grid_width) % span;
        let row = index / grid_width;
        let col = index % grid_width;
        GridLocation { row, col }
    }
}

impl AssetTrait for Project {
    const DIR_NAME: &'static str = "projects";
    const NAME: &'static str = "Project";
    const SHOW_NAME_IF_SELECTED: bool = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::asset::scene::color::SceneInstanceColor;
    use uuid::Uuid;

    fn scene_instance(active: bool, flash: bool) -> SceneInstance {
        SceneInstance {
            id: Uuid::new_v4(),
            name: String::new(),
            color: SceneInstanceColor::default(),
            active,
            opacity: Default::default(),
            input_dimmer: 1.0,
            ignore_main_dimmer: false,
            beat_progression_offset: Default::default(),
            activation_input: None,
            flash_input: None,
            set_offset_on_flash: false,
            dimmer_input: None,
            scene_id: Default::default(),
            scene: Scene::default(),
            groups_overwrite: None,
            palette_overwrite: None,
            flash,
        }
    }

    fn project_with(instance: SceneInstance) -> Project {
        let mut project = Project::default();
        project.add_scene_instance(GridLocation { row: 0, col: 0 }, instance);
        project
    }

    #[test]
    fn keepalive_rate_stays_within_two_to_four_hertz() {
        assert!(OUTPUT_KEEPALIVE_INTERVAL >= Duration::from_millis(250));
        assert!(OUTPUT_KEEPALIVE_INTERVAL <= Duration::from_millis(500));
    }

    #[test]
    fn empty_project_counts_as_unchanged() {
        assert!(!Project::default().output_changed(false));
    }

    #[test]
    fn always_render_counts_as_changed() {
        assert!(Project::default().output_changed(true));
    }

    #[test]
    fn inactive_instance_counts_as_unchanged() {
        assert!(!project_with(scene_instance(false, false)).output_changed(false));
    }

    #[test]
    fn active_instance_counts_as_changed() {
        assert!(project_with(scene_instance(true, false)).output_changed(false));
    }

    #[test]
    fn flashed_instance_counts_as_changed() {
        assert!(project_with(scene_instance(false, true)).output_changed(false));
    }

    #[test]
    fn gate_renders_first_static_tick_then_skips() {
        let mut gate = OutputGate::new();
        let now = Instant::now();
        assert!(gate.should_render(false, now));
        assert!(!gate.should_render(false, now + Duration::from_millis(1)));
    }

    #[test]
    fn gate_falling_edge_renders_once_then_resumes_skipping() {
        let mut gate = OutputGate::new();
        let start = Instant::now();
        for tick in 0..200 {
            assert!(gate.should_render(true, start + Duration::from_millis(tick)));
        }
        assert!(gate.should_render(false, start + Duration::from_millis(200)));
        assert!(!gate.should_render(false, start + Duration::from_millis(201)));
    }

    #[test]
    fn gate_flash_press_then_release_renders_both_edges() {
        let mut gate = OutputGate::new();
        let mut project = project_with(scene_instance(false, false));
        let start = Instant::now();

        assert!(gate.should_render(project.output_changed(false), start));
        assert!(!gate.should_render(
            project.output_changed(false),
            start + Duration::from_millis(1)
        ));

        project
            .scenes_instances_grid
            .values_mut()
            .next()
            .expect("test project contains one scene instance")
            .flash = true;
        assert!(gate.should_render(
            project.output_changed(false),
            start + Duration::from_millis(2)
        ));

        project
            .scenes_instances_grid
            .values_mut()
            .next()
            .expect("test project contains one scene instance")
            .flash = false;
        assert!(gate.should_render(
            project.output_changed(false),
            start + Duration::from_millis(3)
        ));
        assert!(!gate.should_render(
            project.output_changed(false),
            start + Duration::from_millis(4)
        ));
    }

    #[test]
    fn gate_rising_edge_behavior_is_unchanged() {
        let mut gate = OutputGate::new();
        let start = Instant::now();
        assert!(gate.should_render(false, start));
        assert!(!gate.should_render(false, start + Duration::from_millis(1)));
        assert!(gate.should_render(true, start + Duration::from_millis(2)));
        assert!(gate.should_render(true, start + Duration::from_millis(3)));
    }

    #[test]
    fn gate_keepalive_rerenders_three_times_per_second_of_static_ticks() {
        let mut gate = OutputGate::new();
        let start = Instant::now();
        let mut renders = 0;
        let mut last_render = start;
        let mut max_gap = Duration::ZERO;
        // ~120fps tick spacing.
        for tick in 0..125 {
            let now = start + Duration::from_millis(tick * 8);
            if gate.should_render(false, now) {
                renders += 1;
                max_gap = max_gap.max(now - last_render);
                last_render = now;
            }
        }
        assert_eq!(renders, 3);
        assert!(max_gap <= OUTPUT_KEEPALIVE_INTERVAL + Duration::from_millis(8));
    }

    #[test]
    fn gate_long_static_hold_adds_only_one_deactivation_render() {
        let mut gate = OutputGate::new();
        let start = Instant::now();
        let mut renders = 0;
        for tick in 0..1000 {
            let changed = tick == 1;
            if gate.should_render(changed, start + Duration::from_millis(tick)) {
                renders += 1;
            }
        }
        assert_eq!(renders, 5);
    }
}
