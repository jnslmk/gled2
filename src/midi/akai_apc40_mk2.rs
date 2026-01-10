use super::state::MidiState;
use crate::{
    storage::asset::scene::color::SceneInstanceColor,
    ui::action::UiAction,
};
use crossbeam_channel::Receiver;
use midir::MidiOutputConnection;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::storage::asset::project::scene_instance_path::{grid_scene_instance_index, quick_scene_instance_index, SceneInstanceUnion};
use crate::storage::asset::scene::grid::GridLocation;


fn location_from_peripheral_id(index: usize) -> GridLocation {
    let col = index % GRID_HEIGHT;
    let row = index / GRID_WIDTH;
    GridLocation{col, row}
}
pub fn handle_input(_stamp: u64, message: &[u8]) {
    if message.len() != 3 {
        return;
    }

    let status = message[0];
    let peripheral_id = message[1];
    let value = message[2];

    let action = match (status, peripheral_id, value) {
        // scene toggle for non quick scenes
        (144, 0..40, 127) => {
            let index = match peripheral_id {
                0..8 => peripheral_id as usize + 32,
                8..16 => peripheral_id as usize + 16,
                16..24 => peripheral_id as usize,
                24..32 => peripheral_id as usize - 16,
                32..40 => peripheral_id as usize - 32,
                _ => unreachable!(),
            };
            UiAction::ToggleSceneActive(SceneInstanceUnion::Grid(location_from_peripheral_id(index)))
        }
        // scene toggle for quick scenes
        (144..152, 48, 127) => UiAction::ToggleSceneActive(quick_scene_instance_index(status as usize - 144)),
        (144..152, 52, 127) => UiAction::SetSceneActive(
            quick_scene_instance_index(status as usize - 144),
            true,
        ),
        (128..136, 52, 0) => UiAction::SetSceneActive(
            quick_scene_instance_index(status as usize - 128),
            false,
        ),
        (144, 91, 127) => UiAction::SetBlackout(false),
        (128, 91, 127) => UiAction::SetBlackout(true),
        (144, 82..=85 | 99, 127) => UiAction::Tap,
        (176, 13, value) => {
            let delta = if value > 64 {
                value as f32 - 128.0
            } else {
                value as f32
            };
            UiAction::SpeedAdd(delta)
        }
        (176, 14, value) => UiAction::SetMainDimmer(f32::from(value) / 127.0),
        (176..184, 7, value) => UiAction::SetSceneOpacity(
            quick_scene_instance_index(status as usize - 176),
            f32::from(value) / 127.0,
        ),
        (144, 100 | 101, 127) => UiAction::SpeedMultiply(match peripheral_id {
            100 => 0.5,
            101 => 2.0,
            _ => unreachable!(),
        }),
        (176, 48, 0..120) => {
            let value = value as usize / 3;
            UiAction::SelectScene(grid_scene_instance_index(location_from_peripheral_id(value / 3)))
        },
        (176, 49, val) => UiAction::SetSelectedSceneOpacity(f32::from(val) / 127.0),
        _ => {
            //dbg!(status, data1, data2);
            return;
        }
    };

    //dbg!(status, data1, data2, &action);

    action.enqueue();
}

pub fn send_output(state_receiver: Receiver<MidiState>, mut connection: MidiOutputConnection) {
    #[cfg(feature = "profiling")]
    profiling::register_thread!("midi:akai_apc40_mk2:send_output");

    // Reset all lights
    log::trace!("Resetting all lights..");

    for i in 48..56 {
        if let Err(err) = connection.send(&[0x90, 176, i, 0]) {
            log::error!("Could not send value: {err:?}")
        }
    }

    for on in [true, false] {
        for j in 0x90..0x99 {
            for i in 0..127 {
                if let Err(err) = connection.send(&[j, i, if on { 30 } else { 0 }, 127]) {
                    log::error!("Could not send value: {err:?}")
                }
                std::thread::sleep(Duration::from_micros(200));
            }
        }
    }
    log::trace!("Done resetting all lights!");

    log::trace!("Waiting for first state of state receiver..");
    for state in state_receiver {
        if let Err(err) =
            connection.send(&[0x90, 176, 49, (state.selected_scene_opacity * 127.0) as u8])
        {
            log::error!("Could not send value: {err:?}")
        }

        if let Err(err) = connection.send(&[0x90, 91, if state.blackout { 0 } else { 127 }, 127]) {
            log::error!("Error sending blackout value: {err:?}")
        }

        for flank in 0..4 {
            if let Err(err) = connection.send(&[
                0x90,
                82 + flank,
                match (state.beat_flank == flank, state.blackout) {
                    (true, false) => 30,
                    (_, true)
                        if SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_millis() / 200 % 2 == 0)
                            .unwrap_or_default() =>
                    {
                        120
                    }
                    _ => 0,
                },
                127,
            ]) {
                log::error!("Error sending flank value: {err:?}");
            }
        }
        // iterate over all grid locations
        for location in { 0..48 }
            .map(|index| location_from_peripheral_id( index))
        {
            let value = {
                let flashed = state.flashed_scenes.contains(&location);
                let active = state.active_scenes.contains(&location);

                const QUICK_ROW_INDEX: usize = GRID_HEIGHT - 1;
                match location.row {
                    QUICK_ROW_INDEX if active => 30,
                    QUICK_ROW_INDEX => 0,
                    _ => {
                        let color = state.available_scenes_grid.get(&location).copied();
                        match (color, active, flashed) {
                            (_, _, true) => SceneInstanceColor::White.light(),
                            (Some(color), true, false) => color.light(),
                            (Some(color), false, false) => color.dark(),
                            _ => SceneInstanceColor::Black.dark(),
                        }
                    }
                }
            };

            let message = match location.row {
                (0..8) => [0x90, (location.col + location.row *8) as u8 + 32, value, 127],
                (8..16) => [0x90, (location.col + location.row *8) as u8 + 16, value, 127],
                (16..24) => [0x90, (location.col + location.row *8) as u8, value, 127],
                (24..32) => [0x90, (location.col + location.row *8)as u8 - 16, value, 127],
                (32..40) => [0x90, (location.col + location.row *8) as u8 - 32, value, 127],
                // TODO what is the right offset of quick scenes?
                _ => [0x90 + (location.col + location.row *8) as u8, 48, value, 127],
                // _ => unreachable!(),
            };
            if let Err(err) = connection.send(&message) {
                log::error!("Error sending scene value: {err:?}");
            }
        }

        log::trace!("Waiting for next state of state receiver..");
    }
}

trait AkaiApc40Mk2MidiColor {
    fn dark(self) -> u8;
    fn light(self) -> u8;
}

impl AkaiApc40Mk2MidiColor for SceneInstanceColor {
    fn dark(self) -> u8 {
        match self {
            SceneInstanceColor::Red => 7,
            SceneInstanceColor::Green => 19,
            SceneInstanceColor::Blue => 47,
            SceneInstanceColor::White => 1,
            SceneInstanceColor::Orange => 62,
            SceneInstanceColor::Yellow => 15,
            SceneInstanceColor::Purple => 53,
            SceneInstanceColor::Pink => 59,
            SceneInstanceColor::Black => 0,
        }
    }

    fn light(self) -> u8 {
        match self {
            SceneInstanceColor::Red => 5,
            SceneInstanceColor::Green => 17,
            SceneInstanceColor::Blue => 45,
            SceneInstanceColor::White => 3,
            SceneInstanceColor::Orange => 60,
            SceneInstanceColor::Yellow => 13,
            SceneInstanceColor::Purple => 51,
            SceneInstanceColor::Pink => 57,
            SceneInstanceColor::Black => 0,
        }
    }
}

pub const GRID_WIDTH: usize = 6;
pub const GRID_HEIGHT: usize = 4;