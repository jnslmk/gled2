//! Send data to output devices.

use anyhow::{Context, Result};
use kanal::{Receiver, Sender, bounded};
use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    net::SocketAddr,
    sync::Arc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, trace, warn};

use crate::{
    input::artnet::ARTNET_SOCKET,
    pipeline::constants::{UNIVERSE_BUFFER_SIZE, UNIVERSES},
    storage::{
        asset::output_device::{enttec_usb_pro, routing::OutputRoutings},
        collections::Collections,
    },
    svg::{measurement_point::Universes, universe_color_channels::UniverseColorChannels},
    ui::windows::{channel_overwrites::ChannelOverwrites, output_routings::HOVERED_OUTPUT_ROUTING},
};

/// Start output thread.
pub fn start(
    output_receiver: Receiver<(Vec<u8>, Arc<Universes>, Arc<OutputRoutings>)>,
) -> Result<Sender<OutputPackage>> {
    enttec_usb_pro::start();

    debug!("Spawning output thread");

    // Sized to several frames' worth of per-universe packets so the prepare
    // thread never blocks mid-frame waiting on the sender; this decouples the
    // two stages and absorbs short UDP send_to stalls without back-pressuring
    // the GPU readback path (which would otherwise drop whole frames).
    let (sender, receiver) = bounded(256);
    thread::Builder::new()
        .name("gled:output:tx".to_owned())
        .spawn(move || {
            merge_and_send_thread(receiver);
        })
        .context("Could not spawn output merge and send thread")?;

    thread::Builder::new()
        .name("gled:output:prepare".to_owned())
        .spawn({
            let sender = sender.clone();
            move || {
                #[cfg(feature = "profiling")]
                profiling::register_thread!("output:tx");

                // Wait for first output before getting collections to avoid blocking
                let _ = output_receiver.recv();

                let mut collections = Collections::default();

                while let Ok((mut output_data, universes, routings)) = output_receiver.recv() {
                    collections.update();

                    trace!("Preparing output data");
                    {
                        let hovered_output_routing = HOVERED_OUTPUT_ROUTING.lock().clone();
                        let mut channel_overwrites = ChannelOverwrites::get();

                        for (universe, values) in universes
                            .iter()
                            .take(UNIVERSES as usize)
                            .zip(output_data.chunks_exact_mut(UNIVERSE_BUFFER_SIZE as usize))
                        {
                            let Some(routing) = routings.get(universe) else {
                                continue;
                            };
                            if Some(routing) == hovered_output_routing.as_ref() {
                                continue;
                            }
                            let Some(device_id) = routing.device else {
                                continue;
                            };
                            UniverseColorChannels::correct(*universe, values);
                            channel_overwrites.overwrite_data(device_id, routing.universe, values);

                            if let Some(recipient) = routing.recipient(&collections) {
                                let mut data = [0u8; UNIVERSE_BUFFER_SIZE as usize];
                                data.copy_from_slice(values);
                                sender
                                    .send(OutputPackage::Gled { recipient, data })
                                    .expect("Could not send output package");
                            }
                        }

                        if let Some(routing) = hovered_output_routing
                            && let Some(recipient) = routing.recipient(&collections)
                        {
                            sender
                                .send(OutputPackage::Hovered { recipient })
                                .expect("Could not send hovered output package");
                        }

                        for (routing, data) in channel_overwrites.overwritten_universes() {
                            if let Some(recipient) = routing.recipient(&collections) {
                                sender
                                    .send(OutputPackage::Gled { recipient, data })
                                    .expect("Could not send overwritten output package");
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
        htp: bool,
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

    // Stale-entry expiry only needs ~1s granularity, so run it on a timer instead
    // of rescanning both caches on every single packet (which is O(n) per packet
    // and dominates the send loop at hundreds of frames × dozens of universes).
    let mut last_expiry = Instant::now();

    loop {
        let package = match receiver.recv() {
            Ok(package) => package,
            Err(err) => {
                warn!("Output package sender disconnected: {err}");
                break;
            }
        };

        let now = Instant::now();
        let (recipient, hovered) = match package {
            OutputPackage::ArtnetInput {
                recipient,
                data,
                htp,
            } => {
                artnet_input_cache.insert(recipient.clone(), (now, (data, htp)));
                (recipient, false)
            }
            OutputPackage::Gled { recipient, data } => {
                match gled_cache.entry(recipient.clone()) {
                    Entry::Occupied(mut entry) => {
                        if entry.get().2 == [0u8; UNIVERSE_BUFFER_SIZE as usize]
                            && data == [0u8; UNIVERSE_BUFFER_SIZE as usize]
                        {
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

        if last_expiry.elapsed() >= Duration::from_millis(250) {
            last_expiry = Instant::now();
            artnet_input_cache.retain(|_, (last_data, ..)| last_data.elapsed().as_secs() < 1);
            gled_cache.retain(|_, (last_data, ..)| last_data.elapsed().as_secs() < 1);
        }

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
                Some((artnet_last_data, (artnet_data, htp))),
                Some((_gled_last_data, gled_last_update, gled_data)),
                false,
            ) => {
                if *htp {
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

                tracing::debug!("Preparing artnet command for universe {universe}");
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
                    trace!("Sending package to {addr}");
                    trace!("Package data: {data:02x?}");

                    match ARTNET_SOCKET.send_to(&data, addr) {
                        Err(err) if count == 10 => {
                            warn!("Could not send data on try {count}, giving up - {err:?}");
                            break;
                        }
                        Err(err) => {
                            trace!("Could not send data on try {count} - {err:?}");
                            // Yield (not sleep): std::thread::sleep(1ns) rounds up
                            // to a ~50µs nanosleep on Linux, which throttled the
                            // sender to ~5k packets/s. yield_now lets the kernel
                            // drain the send buffer without that fixed penalty.
                            std::thread::yield_now();
                        }
                        Ok(count) => {
                            trace!("Sent data to {addr}");
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
