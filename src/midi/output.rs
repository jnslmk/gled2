use crate::{
    midi::{monitor, runtime::RuntimeBus, state::new_receiver},
    ui::action::UiAction,
};
use midir::MidiOutput;
use std::{
    collections::HashSet,
    thread::{sleep, spawn},
    time::Duration,
};

fn is_own_port(name: &str) -> bool {
    name.contains("gled_read_input") || name.contains("gled_write_output")
}

fn is_midi_through(name: &str) -> bool {
    name.contains("Midi Through")
}

pub fn discover(runtime_bus: RuntimeBus) {
    #[cfg(feature = "profiling")]
    profiling::register_thread!("midi:output:discover");

    let mut was_active = false;
    let mut handled_devices = HashSet::new();
    let mut output = None;

    loop {
        let ports = {
            let output = output.get_or_insert_with(|| {
                MidiOutput::new("gled_write_output").expect("Could not create midi output")
            });
            output
                .ports()
                .into_iter()
                .filter_map(|port| match output.port_name(&port) {
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
        handled_devices.retain(|name| ports.iter().any(|(_, port_name)| port_name == name));

        for (port, name) in ports {
            let id = port.id();
            if handled_devices.contains(&name) {
                continue;
            }

            if is_own_port(&name) {
                continue;
            }
            if is_midi_through(&name) {
                continue;
            }

            tracing::trace!("Discovered midi output device \"{name}\" at \"{id}\"");
            tracing::info!("Connecting to output of \"{name}\" at \"{id}\"");
            monitor::push_event(&name, "output connected", &[]);
            if let Some(output) = output.take() {
                let mut connection = match output.connect(&port, "gled_write_output") {
                    Ok(connection) => connection,
                    Err(err) => {
                        UiAction::Error(format!("Could not connect to midi output: {err:?}"))
                            .enqueue();
                        monitor::push_event(&name, "output connect failed", &[]);
                        continue;
                    }
                };
                let port_name = name.clone();
                let mut runtime = runtime_bus.runtime();
                spawn(move || {
                    let receiver = new_receiver();

                    for state in receiver {
                        runtime.send_output(&port_name, &state, &mut connection);
                    }
                });
                handled_devices.insert(name);
            }
        }

        match (handled_devices.is_empty(), was_active) {
            (true, true) => {
                was_active = false;
                UiAction::MidiOutputActive(false).enqueue();
            }
            (false, false) => {
                was_active = true;
                UiAction::MidiOutputActive(true).enqueue();
            }
            _ => {}
        }

        sleep(Duration::from_secs(1));
    }
}
