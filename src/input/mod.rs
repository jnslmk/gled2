pub mod artnet;
pub mod event;

use egui::{mutex::Mutex, Context};
use event::{GamepadEvent, InputEvent};
use gilrs::{Axis, Button, Event, Gilrs};
use log::debug;
use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc::Receiver, Arc, OnceLock},
};

static INPUT: OnceLock<Arc<Mutex<Input>>> = OnceLock::new();

#[derive(Debug)]
pub struct Input {
    gilrs: Gilrs,
    ctx: Context,
    artnet_receiver: Receiver<artnet::ArtnetEvent>,
    events: HashMap<InputEvent, u8>,
    new_events: HashSet<InputEvent>,
}

impl Input {
    pub fn init(ctx: &Context) {
        let gilrs = Gilrs::new().expect("Could not initialize gilrs");
        let artnet_receiver = artnet::start_thread();
        let ctx = ctx.clone();

        INPUT
            .set(Arc::new(Mutex::new(Self {
                gilrs,
                ctx,
                artnet_receiver,
                events: HashMap::new(),
                new_events: HashSet::new(),
            })))
            .map_err(|_| {})
            .expect("Could not init Input singleton");
    }

    fn get() -> Arc<Mutex<Self>> {
        INPUT
            .get()
            .expect("Input singleton not initialized")
            .clone()
    }

    pub fn tick() {
        let input = Self::get();
        let mut input = input.lock();

        input.new_events.clear();

        if !input.ctx.wants_keyboard_input() {
            let keys_down = input.ctx.input(|i| i.keys_down.clone());
            for key in keys_down.iter().copied() {
                if input.events.insert(InputEvent::Key(key), 255).is_none() {
                    input.new_events.insert(InputEvent::Key(key));
                }
            }
            input.events.retain(|event, _value| {
                if let InputEvent::Key(key) = event {
                    keys_down.contains(key)
                } else {
                    true
                }
            });
        }

        while let Ok(event) = input.artnet_receiver.try_recv() {
            if event.value == 0 {
                input.events.remove(&InputEvent::Artnet(event.channel));
            } else {
                input
                    .events
                    .insert(InputEvent::Artnet(event.channel), event.value);
            }
        }

        while let Some(action) = input.gilrs.next_event().map(
            |Event {
                 id, event, time, ..
             }| {
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
                            GamepadAction::Press(button, (value * 300.0).min(256.0) as u8)
                        } else {
                            GamepadAction::Release(button)
                        }
                    }
                    gilrs::EventType::AxisChanged(axis, value, _) => {
                        // * 300.0 as some gamepads do not reach 1.0 otherwise
                        let value = (value * 300.0).clamp(-256.0, 256.0) as i16;
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
                                return GamepadAction::None;
                            }
                        };

                        if value < -100 {
                            GamepadAction::PressRelease {
                                press: button_1,
                                release: button_2,
                                value: value.unsigned_abs() as u8,
                            }
                        } else if value > 100 {
                            GamepadAction::PressRelease {
                                press: button_2,
                                release: button_1,
                                value: value as u8,
                            }
                        } else {
                            GamepadAction::Release2(button_1, button_2)
                        }
                    }
                    _ => GamepadAction::None,
                }
            },
        ) {
            match action {
                GamepadAction::None => (),
                GamepadAction::Press(event, value) => {
                    input.events.insert(InputEvent::Gamepad(event), value);
                    input.new_events.insert(InputEvent::Gamepad(event));
                }
                GamepadAction::Release(event) => {
                    input.events.remove(&InputEvent::Gamepad(event));
                }
                GamepadAction::PressRelease {
                    press,
                    release,
                    value,
                } => {
                    input.events.insert(InputEvent::Gamepad(press), value);
                    input.new_events.insert(InputEvent::Gamepad(press));
                    input.events.remove(&InputEvent::Gamepad(release));
                }
                GamepadAction::Release2(button_1, button_2) => {
                    input.events.remove(&InputEvent::Gamepad(button_1));
                    input.events.remove(&InputEvent::Gamepad(button_2));
                }
            }
        }
    }

    pub fn first_new_event() -> Option<InputEvent> {
        let input = Self::get();
        let input = input.lock();

        input.new_events.iter().next().copied()
    }

    pub fn is_event_new(event: &InputEvent) -> bool {
        let input = Self::get();
        let input = input.lock();

        input.new_events.contains(event)
    }

    pub fn event_live_value(event: &InputEvent) -> Option<u8> {
        let input = Self::get();
        let input = input.lock();

        input.events.get(event).copied()
    }
}

enum GamepadAction {
    None,
    Press(GamepadEvent, u8),
    Release(GamepadEvent),
    PressRelease {
        press: GamepadEvent,
        release: GamepadEvent,
        value: u8,
    },
    Release2(GamepadEvent, GamepadEvent),
}
