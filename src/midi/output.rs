use crate::{
    midi::{apc40_mk2, state::new_receiver},
    ui::action::UiAction,
};
use midir::MidiOutput;
use std::{
    collections::HashSet,
    thread::{sleep, spawn},
    time::Duration,
};

pub fn discover() {
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
                    Ok(name) => match name.split(':').next() {
                        Some(name) => Some((port, name.to_owned())),
                        None => {
                            log::error!("Midi port name is empty");
                            None
                        }
                    },
                    Err(err) => {
                        log::error!("Could not get midi port name: {err:?}");
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        handled_devices.retain(|id| ports.iter().any(|(port, _)| port.id() == *id));

        for (port, name) in ports {
            let id = port.id();
            if handled_devices.contains(&id) {
                continue;
            }

            log::trace!("Discovered midi output device \"{name}\" at \"{id}\"");
            match name.as_str() {
                "APC40 mkII" => {
                    log::info!("Connecting to output of \"{name}\" at \"{id}\"");
                    if let Some(output) = output.take() {
                        let connection = match output.connect(&port, "gled_write_output") {
                            Ok(connection) => connection,
                            Err(err) => {
                                log::error!("Could not connect to midi output: {err:?}");
                                continue;
                            }
                        };
                        spawn(move || apc40_mk2::send_output(new_receiver(), connection));
                        handled_devices.insert(id);
                    }
                }
                name => {
                    log::trace!("Ignoring \"{name}\" at \"{id}\"")
                }
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
