//! Send data to output devices.

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use log::{debug, trace, warn};
use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    net::SocketAddr,
    thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    input::artnet::ARTNET_SOCKET,
    pipeline::{
        constants::{UNIVERSE_BUFFER_SIZE, UNIVERSES},
        extract_output::ExtractOutput,
    },
    storage::asset::{Asset, output_device::enttec_usb_pro},
    svg::universe_color_channels::UniverseColorChannels,
    ui::windows::{channel_overwrites::ChannelOverwrites, output_routings::HOVERED_OUTPUT_ROUTING},
};

/// Start output thread.
pub fn start() -> Result<Sender<OutputPackage>> {
    enttec_usb_pro::start();

    debug!("Spawning output thread");

    let (sender, receiver) = crossbeam_channel::bounded(10);
    thread::Builder::new()
        .name("gled:output:tx".to_owned())
        .spawn(move || {
            merge_and_send_thread(receiver);
        })
        .context("Could not spawn output merge and send thread")?;

    thread::Builder::new()
        .name("gled:output:tx".to_owned())
        .spawn({
            let sender = sender.clone();
            move || {
                #[cfg(feature = "profiling")]
                profiling::register_thread!("output:tx");

                let extract_output = ExtractOutput::get();
                let output_receiver = extract_output
                    .take_output_receiver()
                    .expect("Could not take output receiver");

                for mut output_data in output_receiver.iter() {
                    trace!("Sending output data");
                    {
                        let mut routings = extract_output.routings.lock();
                        let hovered_output_routing = HOVERED_OUTPUT_ROUTING.lock().clone();
                        let mut channel_overwrites = ChannelOverwrites::get();

                        for (universe, values) in extract_output
                            .universes
                            .lock()
                            .iter()
                            .take(UNIVERSES as usize)
                            .zip(output_data.chunks_exact_mut(UNIVERSE_BUFFER_SIZE as usize))
                        {
                            let routing = routings.universe_output_routing(*universe);
                            if Some(&*routing) == hovered_output_routing.as_ref() {
                                continue;
                            }
                            let Some(device) = routing.device.and_then(Asset::get) else {
                                continue;
                            };
                            UniverseColorChannels::correct(*universe, values);
                            channel_overwrites.overwrite_data(device.id, routing.universe, values);

                            if let Some(recipient) = device.data.get_recipient(routing.universe) {
                                let mut data = [0u8; UNIVERSE_BUFFER_SIZE as usize];
                                data.copy_from_slice(values);
                                sender.send(OutputPackage::Gled { recipient, data }).ok();
                            }
                        }

                        if let Some(routing) = hovered_output_routing
                            && let Some(device) = routing.device.and_then(Asset::get)
                            && let Some(recipient) = device.data.get_recipient(routing.universe)
                        {
                            sender.send(OutputPackage::Hovered { recipient }).ok();
                        }

                        for (device, universe, data) in channel_overwrites.other_universes() {
                            if let Some(device) = Asset::get(device)
                                && let Some(recipient) = device.data.get_recipient(universe)
                            {
                                sender.send(OutputPackage::Gled { recipient, data }).ok();
                            }
                        }
                    }
                }
            }
        })
        .context("Could not spawn artnet thread")?;

    Ok(sender)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Recipient {
    Artnet { addr: SocketAddr, universe: u16 },
    EnttecDmxUsbPro { serial_number: String },
}

pub enum OutputPackage {
    ArtnetInput {
        recipient: Recipient,
        data: [u8; UNIVERSE_BUFFER_SIZE as usize],
        merge: bool,
    },
    Gled {
        recipient: Recipient,
        data: [u8; UNIVERSE_BUFFER_SIZE as usize],
    },
    Hovered {
        recipient: Recipient,
    },
}

fn merge_and_send_thread(receiver: Receiver<OutputPackage>) {
    #[cfg(feature = "profiling")]
    profiling::register_thread!("output:merge_and_send");

    match ARTNET_SOCKET.set_broadcast(true) {
        Ok(_) => debug!("Activated sending to broadcast"),
        Err(e) => debug!("Could not activate sending to broadcast: {e}"),
    }
    match ARTNET_SOCKET.set_nonblocking(true) {
        Ok(_) => debug!("Activated non-blocking mode"),
        Err(e) => debug!("Could not activate non-blocking mode: {e}"),
    };

    let mut artnet_input_cache = HashMap::new();
    let mut gled_cache =
        HashMap::<Recipient, (Instant, Instant, [u8; UNIVERSE_BUFFER_SIZE as usize])>::new();

    for package in receiver.iter() {
        let now = Instant::now();
        let (recipient, hovered) = match package {
            OutputPackage::ArtnetInput {
                recipient,
                data,
                merge,
            } => {
                artnet_input_cache.insert(recipient.clone(), (now, (data, merge)));
                (recipient, false)
            }
            OutputPackage::Gled { recipient, data } => {
                match gled_cache.entry(recipient.clone()) {
                    Entry::Occupied(mut entry) => {
                        if entry.get().2 == data {
                            entry.get_mut().0 = Instant::now();
                        } else {
                            entry.insert((now, now, data));
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert((now, now, data));
                    }
                }
                (recipient, false)
            }
            OutputPackage::Hovered { recipient } => (recipient, true),
        };

        artnet_input_cache.retain(|_, (last_data, ..)| last_data.elapsed().as_secs() < 1);
        gled_cache.retain(|_, (last_data, ..)| last_data.elapsed().as_secs() < 1);

        let data = match (
            artnet_input_cache.get(&recipient),
            gled_cache.get(&recipient),
            hovered,
        ) {
            (_, _, true) => {
                let value = (SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time problem")
                    .as_millis() as i64
                    % 512
                    - 256)
                    .unsigned_abs() as u8;
                Cow::Owned([value; UNIVERSE_BUFFER_SIZE as usize])
            }
            (None, None, false) => continue,
            (None, Some((_last_data, _last_update, data)), false) => Cow::Borrowed(data),
            (Some((_last_data, (data, _merge))), None, false) => Cow::Borrowed(data),
            (
                Some((artnet_last_data, (artnet_data, merge))),
                Some((_gled_last_data, gled_last_update, gled_data)),
                false,
            ) => {
                if *merge {
                    let mut data = [0u8; UNIVERSE_BUFFER_SIZE as usize];
                    for i in 0..UNIVERSE_BUFFER_SIZE as usize {
                        data[i] = gled_data[i].max(artnet_data[i]);
                    }
                    Cow::Owned(data)
                } else if artnet_last_data > gled_last_update {
                    Cow::Borrowed(artnet_data)
                } else {
                    Cow::Borrowed(gled_data)
                }
            }
        };

        match recipient {
            Recipient::Artnet { addr, universe } => {
                let Ok(port_address) = artnet_protocol::PortAddress::try_from(universe) else {
                    warn!("Could not convert universe {universe} to port address");
                    continue;
                };

                log::debug!("Preparing artnet command for universe {universe}");
                let output = artnet_protocol::Output {
                    data: artnet_protocol::PaddedData::from(data.to_vec()),
                    port_address,
                    ..Default::default()
                };

                let Ok(data) = artnet_protocol::ArtCommand::Output(output).write_to_buffer() else {
                    warn!("Could not create artnet output command for universe {universe}");
                    continue;
                };

                let mut count = 0;
                loop {
                    count += 1;
                    debug!("Sending package to {addr}");
                    trace!("Package data: {data:02x?}");

                    match ARTNET_SOCKET.send_to(&data, addr) {
                        Err(err) if count == 10 => {
                            warn!("Could not send data on try {count}, giving up - {err:?}");
                            break;
                        }
                        Err(err) => {
                            warn!("Could not send data on try {count} - {err:?}");
                            std::thread::sleep(std::time::Duration::from_nanos(1));
                        }
                        Ok(count) => {
                            debug!("Sent data to {addr}");
                            crate::network_stats::add_outgoing_bytes(count);
                            break;
                        }
                    };
                }
            }
            Recipient::EnttecDmxUsbPro { serial_number } => {
                enttec_usb_pro::send(serial_number.to_owned(), *data);
            }
        }
    }
}
