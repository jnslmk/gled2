use crate::artnet_receiver::ArtnetEvent;
use egui::{Context, Key, Modifiers};
use gilrs::{Axis, Button, Event, Gilrs};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::mpsc::Receiver};
use strum::Display;

pub struct Gamepad {
    gilrs: Gilrs,
    events: HashSet<GamepadEvent>,
    new_events: HashSet<GamepadEvent>,
    artnet_events: HashSet<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Hotkey {
    Key(Key),
    Gamepad(GamepadEvent),
    Artnet(u8),
}

impl Hotkey {
    pub fn get(ctx: &Context, gamepad: &Gamepad) -> Option<Self> {
        let keys: Vec<Key> = ctx.input_mut(|i| i.keys_down.drain().collect());
        let keys_ = keys.len();
        if let Some(key) = keys.into_iter().next().filter(|_| keys_ == 1) {
            return Some(Self::Key(key));
        }

        let events = gamepad.new_events();
        if events.len() == 1 {
            return events.into_iter().next().map(Self::Gamepad);
        }

        if gamepad.artnet_events.len() == 1 {
            return gamepad
                .artnet_events
                .iter()
                .next()
                .copied()
                .map(Self::Artnet);
        }

        None
    }

    pub fn pressed(&self, ctx: &Context, gamepad: &Gamepad) -> bool {
        match self {
            Hotkey::Key(key) => {
                !ctx.wants_keyboard_input()
                    && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, *key))
            }
            Hotkey::Gamepad(event) => gamepad.new_events().contains(event),
            Hotkey::Artnet(channel) => gamepad.artnet_events.contains(channel),
        }
    }

    pub fn live(&self, ctx: &Context, gamepad: &Gamepad) -> bool {
        match self {
            Hotkey::Key(key) => !ctx.wants_keyboard_input() && ctx.input(|i| i.key_down(*key)),
            Hotkey::Gamepad(event) => gamepad.events().contains(event),
            Hotkey::Artnet(channel) => gamepad.artnet_events.contains(channel),
        }
    }
}

impl std::fmt::Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Hotkey::Key(key) => format!("{key:?}"),
            Hotkey::Gamepad(event) => format!("{event}"),
            Hotkey::Artnet(channel) => format!("ch {channel}"),
        })
    }
}

#[derive(Serialize, Deserialize, Display, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadEvent {
    South(usize),
    East(usize),
    North(usize),
    West(usize),
    C(usize),
    Z(usize),
    LeftTrigger(usize),
    LeftTrigger2(usize),
    RightTrigger(usize),
    RightTrigger2(usize),
    Select(usize),
    Start(usize),
    Mode(usize),
    LeftThumb(usize),
    RightThumb(usize),
    DPadUp(usize),
    DPadDown(usize),
    DPadLeft(usize),
    DPadRight(usize),
    Unknown(usize),
    LeftStickXMin(usize),
    LeftStickXMax(usize),
    LeftStickYMin(usize),
    LeftStickYMax(usize),
    LeftZMin(usize),
    LeftZMax(usize),
    RightStickXMin(usize),
    RightStickXMax(usize),
    RightStickYMin(usize),
    RightStickYMax(usize),
    RightZMin(usize),
    RightZMax(usize),
    DPadXMin(usize),
    DPadXMax(usize),
    DPadYMin(usize),
    DPadYMax(usize),
}

impl Gamepad {
    pub fn new() -> Self {
        let gilrs = Gilrs::new().expect("Could not initialize gilrs");
        Self {
            gilrs,
            events: HashSet::new(),
            new_events: HashSet::new(),
            artnet_events: HashSet::new(),
        }
    }

    pub fn tick(&mut self, receiver: &mut Receiver<ArtnetEvent>) {
        while let Ok(event) = receiver.try_recv() {
            match event {
                ArtnetEvent::On { channel } => {
                    self.artnet_events.insert(channel);
                }
                ArtnetEvent::Off { channel } => {
                    self.artnet_events.remove(&channel);
                }
            }
        }

        self.new_events.clear();
        while let Some(action) = self.gilrs.next_event().map(|Event { id, event, time }| {
            let gamepad_id: usize = id.into();

            debug!("{:?} New event from {}: {:?}", time, id, event);
            match event {
                gilrs::EventType::ButtonChanged(button, value, _) => {
                    let button = match button {
                        Button::South => GamepadEvent::South(gamepad_id),
                        Button::East => GamepadEvent::East(gamepad_id),
                        Button::North => GamepadEvent::North(gamepad_id),
                        Button::West => GamepadEvent::West(gamepad_id),
                        Button::C => GamepadEvent::C(gamepad_id),
                        Button::Z => GamepadEvent::Z(gamepad_id),
                        Button::LeftTrigger => GamepadEvent::LeftTrigger(gamepad_id),
                        Button::LeftTrigger2 => GamepadEvent::LeftTrigger2(gamepad_id),
                        Button::RightTrigger => GamepadEvent::RightTrigger(gamepad_id),
                        Button::RightTrigger2 => GamepadEvent::RightTrigger2(gamepad_id),
                        Button::Select => GamepadEvent::Select(gamepad_id),
                        Button::Start => GamepadEvent::Start(gamepad_id),
                        Button::Mode => GamepadEvent::Mode(gamepad_id),
                        Button::LeftThumb => GamepadEvent::LeftThumb(gamepad_id),
                        Button::RightThumb => GamepadEvent::RightThumb(gamepad_id),
                        Button::DPadUp => GamepadEvent::DPadUp(gamepad_id),
                        Button::DPadDown => GamepadEvent::DPadDown(gamepad_id),
                        Button::DPadLeft => GamepadEvent::DPadLeft(gamepad_id),
                        Button::DPadRight => GamepadEvent::DPadRight(gamepad_id),
                        Button::Unknown => GamepadEvent::Unknown(gamepad_id),
                    };
                    if value > 0.5 {
                        Action::Press(button)
                    } else {
                        Action::Release(button)
                    }
                }
                gilrs::EventType::AxisChanged(axis, value, _) => {
                    // * 300.0 as some gamepads do not reach 1.0
                    let value = (value * 300.0).clamp(-255.0, 255.0) as i16;
                    let (button_1, button_2) = match axis {
                        Axis::LeftStickX => (
                            GamepadEvent::LeftStickXMin(gamepad_id),
                            GamepadEvent::LeftStickXMax(gamepad_id),
                        ),
                        Axis::LeftStickY => (
                            GamepadEvent::LeftStickYMin(gamepad_id),
                            GamepadEvent::LeftStickYMax(gamepad_id),
                        ),
                        Axis::LeftZ => (
                            GamepadEvent::LeftZMin(gamepad_id),
                            GamepadEvent::LeftZMax(gamepad_id),
                        ),
                        Axis::RightStickX => (
                            GamepadEvent::RightStickXMin(gamepad_id),
                            GamepadEvent::RightStickXMax(gamepad_id),
                        ),
                        Axis::RightStickY => (
                            GamepadEvent::RightStickYMin(gamepad_id),
                            GamepadEvent::RightStickYMax(gamepad_id),
                        ),
                        Axis::RightZ => (
                            GamepadEvent::RightZMin(gamepad_id),
                            GamepadEvent::RightZMax(gamepad_id),
                        ),
                        Axis::DPadX => (
                            GamepadEvent::DPadXMin(gamepad_id),
                            GamepadEvent::DPadXMax(gamepad_id),
                        ),
                        Axis::DPadY => (
                            GamepadEvent::DPadYMin(gamepad_id),
                            GamepadEvent::DPadYMax(gamepad_id),
                        ),
                        _ => {
                            return Action::None;
                        }
                    };

                    if value < -200 {
                        Action::PressRelease {
                            press: button_1,
                            release: button_2,
                        }
                    } else if value > 200 {
                        Action::PressRelease {
                            press: button_2,
                            release: button_1,
                        }
                    } else {
                        Action::Release2(button_1, button_2)
                    }
                }
                _ => Action::None,
            }
        }) {
            match action {
                Action::None => (),
                Action::Press(event) => {
                    self.events.insert(event);
                    self.new_events.insert(event);
                }
                Action::Release(event) => {
                    self.events.remove(&event);
                }
                Action::PressRelease { press, release } => {
                    self.events.insert(press);
                    self.new_events.insert(press);
                    self.events.remove(&release);
                }
                Action::Release2(button_1, button_2) => {
                    self.events.remove(&button_1);
                    self.events.remove(&button_2);
                }
            }
        }
    }

    pub fn new_events(&self) -> HashSet<GamepadEvent> {
        self.new_events.clone()
    }

    pub fn events(&self) -> HashSet<GamepadEvent> {
        self.events.clone()
    }
}

enum Action {
    None,
    Press(GamepadEvent),
    Release(GamepadEvent),
    PressRelease {
        press: GamepadEvent,
        release: GamepadEvent,
    },
    Release2(GamepadEvent, GamepadEvent),
}
