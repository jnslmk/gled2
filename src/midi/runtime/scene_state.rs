use crate::{
    midi::state::MidiState,
    storage::asset::{
        midi_controller::MidiSceneTarget,
        project::scene_instance_path::{
            SceneInstanceUnion, grid_scene_instance_index, quick_scene_instance_index,
        },
        scene::color::SceneInstanceColor,
    },
};

pub(super) fn scene_target_to_union(target: &MidiSceneTarget) -> Option<SceneInstanceUnion> {
    match target {
        MidiSceneTarget::Selected => Some(SceneInstanceUnion::Selected),
        MidiSceneTarget::Quick { index } => Some(quick_scene_instance_index(*index as usize)),
        MidiSceneTarget::Grid { row, col } => Some(grid_scene_instance_index(
            crate::storage::asset::scene::grid::GridLocation::new(*col as usize, *row as usize),
        )),
    }
}

fn target_location(
    state: &MidiState,
    target: &MidiSceneTarget,
) -> crate::storage::asset::scene::grid::GridLocation {
    match target {
        MidiSceneTarget::Selected => state.selected_scene_location,
        MidiSceneTarget::Quick { index } => {
            if let Some(row) = state.highlighted_row {
                crate::storage::asset::scene::grid::GridLocation::new(*index as usize, row)
            } else if let Some(col) = state.highlighted_col {
                crate::storage::asset::scene::grid::GridLocation::new(col, *index as usize)
            } else {
                crate::storage::asset::scene::grid::GridLocation::new(*index as usize, 0)
            }
        }
        MidiSceneTarget::Grid { row, col } => {
            crate::storage::asset::scene::grid::GridLocation::new(*col as usize, *row as usize)
        }
    }
}

pub(super) fn scene_color(state: &MidiState, target: &MidiSceneTarget) -> SceneInstanceColor {
    state
        .available_scenes_grid
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(SceneInstanceColor::Black)
}

pub(super) fn scene_is_active(state: &MidiState, target: &MidiSceneTarget) -> bool {
    state
        .active_scenes
        .contains(&target_location(state, target))
}

pub(super) fn scene_is_flashed(state: &MidiState, target: &MidiSceneTarget) -> bool {
    state
        .flashed_scenes
        .contains(&target_location(state, target))
}

pub(super) fn scene_opacity(state: &MidiState, target: &MidiSceneTarget) -> f32 {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_opacity;
    }

    state
        .scene_opacity
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(0.0)
}

pub(super) fn scene_input_dimmer(state: &MidiState, target: &MidiSceneTarget) -> f32 {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_input_dimmer;
    }

    state
        .scene_input_dimmer
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(1.0)
}

pub(super) fn scene_beat_offset(state: &MidiState, target: &MidiSceneTarget) -> f32 {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_beat_offset;
    }

    state
        .scene_beat_offset
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(0.0)
}

pub(super) fn scene_ignore_main_dimmer(state: &MidiState, target: &MidiSceneTarget) -> bool {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_ignore_main_dimmer;
    }

    state
        .scene_ignore_main_dimmer
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(false)
}

pub(super) fn scene_set_offset_on_flash(state: &MidiState, target: &MidiSceneTarget) -> bool {
    if matches!(target, MidiSceneTarget::Selected) {
        return state.selected_scene_set_offset_on_flash;
    }

    state
        .scene_set_offset_on_flash
        .get(&target_location(state, target))
        .copied()
        .unwrap_or(false)
}

pub(super) fn scene_effect_setting_f32(
    state: &MidiState,
    target: &MidiSceneTarget,
    effect_index: usize,
    setting_index: usize,
) -> f32 {
    state
        .scene_effect_setting_f32
        .get(&(target_location(state, target), effect_index, setting_index))
        .copied()
        .unwrap_or(0.0)
}

pub(super) fn scene_effect_opacity(
    state: &MidiState,
    target: &MidiSceneTarget,
    effect_index: usize,
) -> f32 {
    state
        .scene_effect_opacity
        .get(&(target_location(state, target), effect_index))
        .copied()
        .unwrap_or(0.0)
}

pub(super) fn scene_effect_color_shift(
    state: &MidiState,
    target: &MidiSceneTarget,
    effect_index: usize,
) -> f32 {
    state
        .scene_effect_color_shift
        .get(&(target_location(state, target), effect_index))
        .copied()
        .unwrap_or(0.0)
}

pub(super) fn scene_effect_beat_progression(
    state: &MidiState,
    target: &MidiSceneTarget,
    effect_index: usize,
) -> f32 {
    state
        .scene_effect_beat_progression
        .get(&(target_location(state, target), effect_index))
        .copied()
        .unwrap_or(0.0)
}

pub(super) fn scene_effect_beat_offset(
    state: &MidiState,
    target: &MidiSceneTarget,
    effect_index: usize,
) -> f32 {
    state
        .scene_effect_beat_offset
        .get(&(target_location(state, target), effect_index))
        .copied()
        .unwrap_or(0.0)
}

pub(super) fn scene_effect_speed_exponent(
    state: &MidiState,
    target: &MidiSceneTarget,
    effect_index: usize,
) -> f32 {
    state
        .scene_effect_speed_exponent
        .get(&(target_location(state, target), effect_index))
        .copied()
        .unwrap_or(0.5)
}

pub(super) fn scene_effect_group_index(
    state: &MidiState,
    target: &MidiSceneTarget,
    effect_index: usize,
) -> f32 {
    state
        .scene_effect_group_index
        .get(&(target_location(state, target), effect_index))
        .copied()
        .unwrap_or(0.0)
}
