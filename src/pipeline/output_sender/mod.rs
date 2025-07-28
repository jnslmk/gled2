//! Send data to output devices.

pub mod enttec_usb_pro;

use anyhow::{Context, Result};
use log::{debug, trace, warn};
use std::{
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use crate::{
    pipeline::{
        constants::{UNIVERSE_BUFFER_SIZE, UNIVERSES},
        extract_output::ExtractOutput,
    },
    storage::asset::{Asset, output_device::OutputDevice},
    svg::universe_color_channels::UniverseColorChannels,
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
            let socket = { 6000..7000 }
                .filter_map(|port| UdpSocket::bind(("0.0.0.0", port)).ok())
                .next()
                .expect("Could not find a port which we can use");
            match socket.set_broadcast(true) {
                Ok(_) => debug!("Activated sending to broadcast"),
                Err(e) => debug!("Could not activate sending to broadcast: {e}"),
            }
            match socket.set_nonblocking(true) {
                Ok(_) => debug!("Activated non-blocking mode"),
                Err(e) => debug!("Could not activate non-blocking mode: {e}"),
            };

            let extract_output = ExtractOutput::get();

            for use_first_output_buffer in output_receiver.iter() {
                trace!("Sending output data");
                let packages: Vec<(SocketAddr, Vec<u8>)> = {
                    let mut output_data =
                        extract_output.poll_output_buffer(use_first_output_buffer);
                    gpu_ready_sender.send(()).expect("GPU ready receiver lost");

                    let mut routings = extract_output.routings.lock();

                    extract_output
                        .universes
                        .lock()
                        .iter()
                        .take(UNIVERSES as usize)
                        .zip(output_data.chunks_exact_mut(UNIVERSE_BUFFER_SIZE as usize))
                        .filter_map(|(universe, data)| {
                            let routing = routings.universe_output_routing(*universe);
                            let device = routing.device.and_then(Asset::get)?;
                            UniverseColorChannels::correct(*universe, data);

                            match &device.data {
                                OutputDevice::Artnet { ip, universes, .. } => {
                                    let universe = routing.universe?;
                                    if !universes.contains(&universe) {
                                        log::warn!("Universe which is not configured: {universe}");
                                        return None;
                                    }

                                    log::debug!("Preparing artnet command for universe {universe}");
                                    let output = artnet_protocol::Output {
                                        data: artnet_protocol::PaddedData::from(data.to_vec()),
                                        port_address: artnet_protocol::PortAddress::try_from(
                                            universe,
                                        )
                                        .ok()?,
                                        ..Default::default()
                                    };

                                    (*ip, 6454)
                                        .to_socket_addrs()
                                        .ok()
                                        .and_then(|mut addrs| addrs.next())
                                        .and_then(|addr| {
                                            artnet_protocol::ArtCommand::Output(output)
                                                .write_to_buffer()
                                                .ok()
                                                .map(|data| (addr, data))
                                        })
                                }
                                OutputDevice::EnttecDmxUsbPro { serial_number } => {
                                    let mut send_data = [0u8; 512];
                                    send_data[..data.len()].copy_from_slice(data);
                                    enttec_usb_pro::send(serial_number.to_owned(), send_data);
                                    None
                                }
                                OutputDevice::WledDRGB { ip, port, .. } => {
                                    log::debug!("Preparing wled drgb data for universe {universe}");
                                    let mut wled_data = Vec::with_capacity(512);
                                    wled_data.push(2); // DRGB
                                    wled_data.push(255); // Seconds of no signal after which to switch to auto. 255 is infinite.
                                    wled_data.extend(&data[..510]);

                                    (*ip, *port)
                                        .to_socket_addrs()
                                        .ok()
                                        .and_then(|mut addrs| addrs.next())
                                        .map(|addr| (addr, wled_data))
                                }
                                OutputDevice::WledDNRGB {
                                    ip, port, start, ..
                                } => {
                                    log::debug!(
                                        "Preparing wled dnrgb data for universe {universe}"
                                    );
                                    let mut wled_data = Vec::with_capacity(514);
                                    wled_data.push(4); // DNRGB
                                    wled_data.push(255); // Seconds of no signal after which to switch to auto. 255 is infinite.
                                    wled_data.push(start.to_be_bytes()[0]);
                                    wled_data.push(start.to_be_bytes()[1]);
                                    wled_data.extend(&data[..510]);

                                    (*ip, *port)
                                        .to_socket_addrs()
                                        .ok()
                                        .and_then(|mut addrs| addrs.next())
                                        .map(|addr| (addr, wled_data))
                                }
                            }
                        })
                        .collect()
                };

                for (addr, data) in packages {
                    debug!("Sending package to {addr}");
                    trace!("Package data: {data:02x?}");
                    if let Err(err) = socket.send_to(&data, addr) {
                        warn!("Could not send data: {err:?}")
                    };
                }
            }
        })
        .context("Could not spawn artnet thread")?;

    Ok((output_sender, gpu_ready_receiver))
}
