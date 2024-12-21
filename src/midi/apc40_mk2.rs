use crate::{
    storage::{DeckPath, SceneInstancePath},
    ui::action::UiAction,
};

pub fn handle_input(_stamp: u64, message: &[u8]) {
    if message.len() != 3 {
        return;
    }

    let status = message[0];
    let data1 = message[1];
    let data2 = message[2];
    match (status, data1, data2) {
        (144, 0..40, 127) => {
            let path = match data1 {
                0..4 => SceneInstancePath::new(DeckPath::A, data1 as usize + 16),
                4..8 => SceneInstancePath::new(DeckPath::B, data1 as usize + 12),
                8..12 => SceneInstancePath::new(DeckPath::A, data1 as usize + 4),
                12..16 => SceneInstancePath::new(DeckPath::B, data1 as usize),
                16..20 => SceneInstancePath::new(DeckPath::A, data1 as usize - 8),
                20..24 => SceneInstancePath::new(DeckPath::B, data1 as usize - 12),
                24..28 => SceneInstancePath::new(DeckPath::A, data1 as usize - 20),
                28..32 => SceneInstancePath::new(DeckPath::B, data1 as usize - 24),
                32..36 => SceneInstancePath::new(DeckPath::A, data1 as usize - 32),
                36..40 => SceneInstancePath::new(DeckPath::B, data1 as usize - 36),
                _ => unreachable!(),
            };
            UiAction::ToggleSceneActive(path)
        }
        (144, 91, 127) => UiAction::SetBlackout(false),
        (128, 91, 127) => UiAction::SetBlackout(true),
        (144, 99, 127) => UiAction::Tap,
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
            SceneInstancePath::new(DeckPath::C, status as usize - 176),
            f32::from(value) / 127.0,
        ),
        (144, 100 | 101, 127) => UiAction::SpeedMultiply(match data1 {
            100 => 0.5,
            101 => 2.0,
            _ => unreachable!(),
        }),
        (176, 15, value) => UiAction::SetCrossFader(f32::from(value) / 127.0),
        _ => {
            return;
        }
    }
    .enqueue();
}
