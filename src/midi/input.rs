use crate::{
    midi::{learn, monitor, runtime::RuntimeBus},
    ui::action::UiAction,
};
use midir::MidiInput;
use std::{collections::HashMap, thread::sleep, time::Duration};

fn is_own_port(name: &str) -> bool {
    name.contains("gled_read_input") || name.contains("gled_write_output")
}

fn is_midi_through(name: &str) -> bool {
    name.contains("Midi Through")
}

pub fn discover(runtime_bus: RuntimeBus) {
    #[cfg(feature = "profiling")]
    profiling::register_thread!("midi:input:discover");

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
                    Ok(name) => {
                        if name.trim().is_empty() {
                            UiAction::Error("Midi port name is empty".to_string()).enqueue();
                            None
                        } else {
                            Some((port, name))
                        }
                    }
                    Err(err) => {
                        UiAction::Error(format!("Could not get midi port name: {err:?}")).enqueue();
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        connections.retain(|name, _| ports.iter().any(|(_, port_name)| port_name == name));

        for (port, name) in ports {
            let id = port.id();
            if connections.contains_key(&name) {
                continue;
            }

            if is_own_port(&name) {
                continue;
            }
            if is_midi_through(&name) {
                continue;
            }

            tracing::trace!("Discovered midi input device \"{name}\" at \"{id}\"");
            tracing::info!("Connecting to input of \"{name}\" at \"{id}\"");
            monitor::push_event(&name, "input connected", &[]);
            if let Some(input) = input.take() {
                let mut runtime = runtime_bus.runtime();
                let connection = {
                    let id = id.clone();
                    let port_name = name.clone();
                    match input.connect(
                        &port,
                        "gled_read_input",
                        move |_stamp, message, _| {
                            tracing::trace!(
                                "Midi message from \"{port_name}\" at \"{id}\": {message:?}"
                            );

                            monitor::push_event(&port_name, "midi", message);

                            let _handled = learn::capture(message)
                                || runtime.handle_input(&port_name, message);
                        },
                        (),
                    ) {
                        Ok(connection) => connection,
                        Err(err) => {
                            UiAction::Error(format!("Could not connect to midi input: {err:?}"))
                                .enqueue();
                            monitor::push_event(&name, "input connect failed", &[]);
                            continue;
                        }
                    }
                };
                connections.insert(name, connection);
            }
        }
        sleep(Duration::from_secs(1));
    }
}
