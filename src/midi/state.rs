use crate::output_state::ProjectState;
use crate::storage::asset::scene::color::SceneInstanceColor;
use crate::storage::asset::scene::grid::GridLocation;
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, unbounded};
use once_cell::sync::{Lazy, OnceCell};
use std::collections::{HashMap, HashSet};

static SENDER: OnceCell<Sender<MidiState>> = OnceCell::new();
static SENDERS: Lazy<Mutex<Vec<Sender<MidiState>>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MidiState {
    pub blackout: bool,
    pub beat_progression: f32,
    pub beats_per_minute: f32,
    pub beat_flank: u8,
    pub main_dimmer: f32,
    pub highlighted_row: Option<usize>,
    pub highlighted_col: Option<usize>,
    pub selected_scene_location: GridLocation,
    pub active_scenes: HashSet<GridLocation>,
    pub flashed_scenes: HashSet<GridLocation>,
    pub available_scenes_grid: HashMap<GridLocation, SceneInstanceColor>,
    pub selected_scene_color: SceneInstanceColor,
    pub selected_scene_opacity: f32,
    pub selected_scene_input_dimmer: f32,
    pub selected_scene_beat_offset: f32,
    pub selected_scene_ignore_main_dimmer: bool,
    pub selected_scene_set_offset_on_flash: bool,
    pub scene_input_dimmer: HashMap<GridLocation, f32>,
    pub scene_beat_offset: HashMap<GridLocation, f32>,
    pub scene_ignore_main_dimmer: HashMap<GridLocation, bool>,
    pub scene_set_offset_on_flash: HashMap<GridLocation, bool>,
    pub scene_effect_setting_f32: HashMap<(GridLocation, usize, usize), f32>,
}
impl Eq for MidiState {}

impl MidiState {
    fn beat_flank_from_progression(beat_progression: f32) -> u8 {
        ((beat_progression.rem_euclid(1.0) * 4.0).floor() as u8).min(4)
    }

    fn float_value_to_f32(
        value: &crate::storage::asset::animation::config::float_value::FloatValue,
    ) -> f32 {
        match value {
            crate::storage::asset::animation::config::float_value::FloatValue::F32(value) => *value,
            crate::storage::asset::animation::config::float_value::FloatValue::Percentage(
                curve,
            ) => curve.multiplier,
            crate::storage::asset::animation::config::float_value::FloatValue::Degrees(curve) => {
                curve.multiplier
            }
        }
    }

    fn from_project_state(state: &ProjectState) -> Self {
        let mut scene_input_dimmer = HashMap::new();
        let mut scene_beat_offset = HashMap::new();
        let mut scene_ignore_main_dimmer = HashMap::new();
        let mut scene_set_offset_on_flash = HashMap::new();
        let mut scene_effect_setting_f32 = HashMap::new();

        let project = state.project.as_ref();
        let (highlighted_row, highlighted_col) = if let Some(project) = project {
            match project.grid_highlight {
                crate::storage::asset::project::GridHighlight::Row => {
                    (Some(project.quick_row_index()), None)
                }
                crate::storage::asset::project::GridHighlight::Column => {
                    (None, Some(project.grid_width().saturating_sub(1)))
                }
                crate::storage::asset::project::GridHighlight::None => (None, None),
            }
        } else {
            (None, None)
        };

        if let Some(project) = project {
            for (location, scene_instance) in &project.scenes_instances_grid {
                scene_input_dimmer.insert(*location, scene_instance.input_dimmer);
                scene_beat_offset
                    .insert(*location, scene_instance.beat_progression_offset.multiplier);
                scene_ignore_main_dimmer.insert(*location, scene_instance.ignore_main_dimmer);
                scene_set_offset_on_flash.insert(*location, scene_instance.set_offset_on_flash);

                for (effect_index, effect) in scene_instance.scene.effects.iter().enumerate() {
                    let values = [
                        &effect.animation_config.float_0,
                        &effect.animation_config.float_1,
                        &effect.animation_config.float_2,
                        &effect.animation_config.float_3,
                        &effect.animation_config.float_4,
                        &effect.animation_config.float_5,
                    ];
                    for (setting_index, value) in values.into_iter().enumerate() {
                        scene_effect_setting_f32.insert(
                            (*location, effect_index, setting_index),
                            Self::float_value_to_f32(value),
                        );
                    }
                }
            }
        }

        let selected_scene = project.and_then(|project| {
            project
                .scenes_instances_grid
                .get(&state.selected_scene_instance)
        });

        MidiState {
            blackout: state.blackout,
            beat_progression: state.beat_progression,
            beats_per_minute: state.beats_per_minute,
            beat_flank: Self::beat_flank_from_progression(state.beat_progression),
            main_dimmer: project.map_or(1.0, |project| project.main_dimmer),
            highlighted_row,
            highlighted_col,
            selected_scene_location: state.selected_scene_instance,
            active_scenes: project.map_or_else(Default::default, |project| {
                project
                    .scenes_instances_grid
                    .iter()
                    .filter_map(
                        |(location, scene)| if scene.active { Some(*location) } else { None },
                    )
                    .collect()
            }),
            flashed_scenes: project.map_or_else(Default::default, |project| {
                project
                    .scenes_instances_grid
                    .iter()
                    .filter_map(
                        |(location, scene)| if scene.flash { Some(*location) } else { None },
                    )
                    .collect()
            }),
            available_scenes_grid: project.map_or_else(Default::default, |project| {
                project
                    .scenes_instances_grid
                    .iter()
                    .map(|(location, scene_instance)| (*location, scene_instance.color))
                    .collect()
            }),
            selected_scene_color: selected_scene
                .map_or(SceneInstanceColor::Black, |scene| scene.color),
            selected_scene_opacity: selected_scene.map_or(1.0, |scene| scene.opacity.multiplier),
            selected_scene_input_dimmer: selected_scene.map_or(1.0, |scene| scene.input_dimmer),
            selected_scene_beat_offset: selected_scene
                .map_or(0.0, |scene| scene.beat_progression_offset.multiplier),
            selected_scene_ignore_main_dimmer: selected_scene
                .is_some_and(|scene| scene.ignore_main_dimmer),
            selected_scene_set_offset_on_flash: selected_scene
                .is_some_and(|scene| scene.set_offset_on_flash),
            scene_input_dimmer,
            scene_beat_offset,
            scene_ignore_main_dimmer,
            scene_set_offset_on_flash,
            scene_effect_setting_f32,
        }
    }
}

pub fn start() {
    #[cfg(feature = "profiling")]
    profiling::register_thread!("midi:state");

    let (sender, _receiver) = unbounded();
    SENDER.set(sender).expect("Could not set sender");

    let state_receiver = crate::output_state::new_receiver();

    let mut previous = MidiState::default();

    loop {
        let state = state_receiver
            .recv()
            .expect("Could not receive project state");
        let midi_state = MidiState::from_project_state(&state);

        if previous == midi_state {
            continue;
        }
        previous = midi_state.clone();
        SENDERS
            .lock()
            .retain(|sender| sender.send(midi_state.clone()).is_ok());
    }
}

pub fn new_receiver() -> Receiver<MidiState> {
    let (sender, receiver) = unbounded();
    SENDERS.lock().push(sender);
    receiver
}
