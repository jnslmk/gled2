use super::state::MidiState;
use crate::{
    storage::asset::project::{DeckPath, scene_instance_path::SceneInstancePathIndex},
    ui::action::UiAction,
};
use crossbeam_channel::Receiver;
use midir::MidiOutputConnection;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn handle_input(_stamp: u64, message: &[u8]) {
    if message.len() != 3 {
        return;
    }

    let status = message[0];
    let data1 = message[1];
    let data2 = message[2];
    match (status, data1, data2) {
        (144, 0..40, 127) => {
            let index = match data1 {
                0..4 => data1 as usize + 16,
                4..8 => data1 as usize + 32,
                8..12 => data1 as usize + 4,
                12..16 => data1 as usize + 20,
                16..20 => data1 as usize - 8,
                20..24 => data1 as usize + 8,
                24..28 => data1 as usize - 20,
                28..32 => data1 as usize - 4,
                32..36 => data1 as usize - 32,
                36..40 => data1 as usize - 16,
                _ => unreachable!(),
            };
            UiAction::ToggleSceneActive(SceneInstancePathIndex::new(DeckPath::Grid, index))
        }
        (128..136, 48 | 52, _) => UiAction::SetSceneActive(
            SceneInstancePathIndex::new(DeckPath::Quick, status as usize - 128),
            false,
        ),
        (144..152, 48 | 52, 127) => UiAction::SetSceneActive(
            SceneInstancePathIndex::new(DeckPath::Quick, status as usize - 144),
            true,
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
            SceneInstancePathIndex::new(DeckPath::Quick, status as usize - 176),
            f32::from(value) / 127.0,
        ),
        (144, 100 | 101, 127) => UiAction::SpeedMultiply(match data1 {
            100 => 0.5,
            101 => 2.0,
            _ => unreachable!(),
        }),
        _ => {
            return;
        }
    }
    .enqueue();
}

pub fn send_output(state_receiver: Receiver<MidiState>, mut connection: MidiOutputConnection) {
    // Reset all lights
    log::trace!("Resetting all lights..");
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

        for path in { 0..40 }
            .map(|index| SceneInstancePathIndex::new(DeckPath::Grid, index))
            .chain({ 0..8 }.map(|index| SceneInstancePathIndex::new(DeckPath::Quick, index)))
        {
            let active = state.active_scenes.contains(&path);
            let value = match (path.deck_path, active) {
                (DeckPath::Grid, true) => 30,
                (DeckPath::Grid, false) if state.available_scenes_grid > path.index => 11,
                (DeckPath::Quick, true) => 30,
                _ => 0,
            };

            let message = match (path.deck_path, path.index) {
                (DeckPath::Grid, 0..4) => [0x90, path.index as u8 + 32, value, 127],
                (DeckPath::Grid, 4..8) => [0x90, path.index as u8 + 20, value, 127],
                (DeckPath::Grid, 8..12) => [0x90, path.index as u8 + 8, value, 127],
                (DeckPath::Grid, 12..16) => [0x90, path.index as u8 + 4, value, 127],
                (DeckPath::Grid, 16..20) => [0x90, path.index as u8 - 16, value, 127],
                (DeckPath::Grid, 20..24) => [0x90, path.index as u8 + 16, value, 127],
                (DeckPath::Grid, 24..28) => [0x90, path.index as u8 + 4, value, 127],
                (DeckPath::Grid, 28..32) => [0x90, path.index as u8 - 8, value, 127],
                (DeckPath::Grid, 32..36) => [0x90, path.index as u8 - 20, value, 127],
                (DeckPath::Grid, 36..40) => [0x90, path.index as u8 - 32, value, 127],
                (DeckPath::Quick, _) => [0x90 + path.index as u8, 48, value, 127],
                _ => unreachable!(),
            };
            if let Err(err) = connection.send(&message) {
                log::error!("Error sending scene value: {err:?}");
            }
        }

        log::trace!("Waiting for next state of state receiver..");
    }
}
