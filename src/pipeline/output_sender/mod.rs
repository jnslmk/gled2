//! Send data to output devices.

use anyhow::{Context, Result};
use log::{debug, trace, warn};
use std::{
    net::SocketAddr,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
    time::{SystemTime, UNIX_EPOCH},
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
pub type OutputSender = Sender<bool>;
pub type GpuReadySender = Sender<()>;
pub type GpuReadyReceiver = Receiver<()>;

/// Start output thread.
///
/// Returns:
/// * a sender to activate the output thread
/// * a receiver to wait for the GPU to be ready again
pub fn start() -> Result<(OutputSender, GpuReadyReceiver)> {
    enttec_usb_pro::start();

    debug!("Spawning output thread");
    let (output_sender, output_receiver) = channel();
    let (gpu_ready_sender, gpu_ready_receiver) = channel::<()>();
    gpu_ready_sender.send(()).ok();

    thread::Builder::new()
        .name("gled:output:tx".to_owned())
        .spawn(move || {
            match ARTNET_SOCKET.set_broadcast(true) {
                Ok(_) => debug!("Activated sending to broadcast"),
                Err(e) => debug!("Could not activate sending to broadcast: {e}"),
            }
            match ARTNET_SOCKET.set_nonblocking(true) {
                Ok(_) => debug!("Activated non-blocking mode"),
                Err(e) => debug!("Could not activate non-blocking mode: {e}"),
            };

            let extract_output = ExtractOutput::get();

            for use_first_output_buffer in output_receiver.iter() {
                trace!("Sending output data");
                let packages: Vec<(SocketAddr, Vec<u8>)> = {
                    let mut output_data =
                        extract_output.poll_output_buffer(use_first_output_buffer);
                    if gpu_ready_sender.send(()).is_err() {
                        return;
                    }

                    let mut routings = extract_output.routings.lock();
                    let hovered_output_routing = HOVERED_OUTPUT_ROUTING.lock().clone();
                    let mut channel_overwrites = ChannelOverwrites::get();

                    let mut packages = extract_output
                        .universes
                        .lock()
                        .iter()
                        .take(UNIVERSES as usize)
                        .zip(output_data.chunks_exact_mut(UNIVERSE_BUFFER_SIZE as usize))
                        .filter_map(|(universe, data)| {
                            let routing = routings.universe_output_routing(*universe);
                            if Some(&*routing) == hovered_output_routing.as_ref() {
                                return None;
                            }
                            let device = routing.device.and_then(Asset::get)?;
                            UniverseColorChannels::correct(*universe, data);
                            channel_overwrites.overwrite_data(device.id, routing.universe, data);

                            device.data.prepare_package(routing.universe, data)
                        })
                        .collect::<Vec<_>>();
                    if let Some(routing) = hovered_output_routing
                        && let Some(device) = routing.device.and_then(Asset::get)
                    {
                        let value = (SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("time problem")
                            .as_millis() as i64
                            % 512
                            - 256)
                            .unsigned_abs() as u8;
                        if let Some((addr, data)) = device.data.prepare_package(
                            routing.universe,
                            &[value; UNIVERSE_BUFFER_SIZE as usize],
                        ) {
                            packages.push((addr, data));
                        }
                    }
                    for (device, universe, data) in channel_overwrites.other_universes() {
                        if let Some(device) = Asset::get(device)
                            && let Some((addr, data)) = device.data.prepare_package(universe, &data)
                        {
                            packages.push((addr, data));
                        }
                    }
                    packages
                };

                for (addr, data) in packages {
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
                            Ok(_) => {
                                debug!("Sent data to {addr}");
                                break;
                            }
                        };
                    }
                }
            }
        })
        .context("Could not spawn artnet thread")?;

    Ok((output_sender, gpu_ready_receiver))
}
