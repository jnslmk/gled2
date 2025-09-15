use crate::{midi::akai_apc40_mk2, ui::action::UiAction};
use midir::MidiInput;
use std::{collections::HashMap, thread::sleep, time::Duration};

pub fn discover() {
    let mut connections = HashMap::new();
    let mut input = None;

    loop {
        let ports = {
            let input = input.get_or_insert_with(|| {
                MidiInput::new("gled_read_input").expect("Could not create midi input")
            });
            input
                .ports()
                .into_iter()
                .filter_map(|port| match input.port_name(&port) {
                    Ok(name) => match name.split(':').next() {
                        Some(name) => Some((port, name.to_owned())),
                        None => {
                            UiAction::Error("Midi port name is empty".to_string()).enqueue();
                            None
                        }
                    },
                    Err(err) => {
                        UiAction::Error(format!("Could not get midi port name: {err:?}")).enqueue();
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        connections.retain(|id, _| ports.iter().any(|(port, _)| port.id() == *id));

        for (port, name) in ports {
            let id = port.id();
            if connections.contains_key(&id) {
                continue;
            }

            log::trace!("Discovered midi input device \"{name}\" at \"{id}\"");
            match name.as_str() {
                "APC40 mkII" | "APC40 mkII [0]" | "APC40 mkII [1]" => {
                    log::info!("Connecting to input of \"{name}\" at \"{id}\"");
                    if let Some(input) = input.take() {
                        let connection = {
                            let id = id.clone();
                            match input.connect(
                                &port,
                                "gled_read_input",
                                move |stamp, message, _| {
                                    log::trace!(
                                        "Midi message from \"{name}\" at \"{id}\": {message:?}"
                                    );
                                    akai_apc40_mk2::handle_input(stamp, message);
                                },
                                (),
                            ) {
                                Ok(connection) => connection,
                                Err(err) => {
                                    UiAction::Error(format!(
                                        "Could not connect to midi input: {err:?}"
                                    ))
                                    .enqueue();
                                    continue;
                                }
                            }
                        };
                        connections.insert(id, connection);
                    }
                }
                name => {
                    log::trace!("Ignoring \"{name}\" at \"{id}\"")
                }
            }
        }
        sleep(Duration::from_secs(1));
    }
}
