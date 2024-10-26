//! Send data via Art-Net udp protocol.
use anyhow::{Context, Result};
use log::{debug, error};
use std::{
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    sync::mpsc::{Receiver, Sender},
    thread,
};

use crate::{
    constants::{UNIVERSES, UNIVERSE_BUFFER_SIZE},
    extract_output::ExtractOutput,
    project::UniverseOutput,
};
pub type OutputSender = Sender<()>;
pub type GpuReadyReceiver = Receiver<()>;

/// Start output thread
pub fn start() -> Result<(OutputSender, GpuReadyReceiver)> {
    debug!("Spawning output thread");
    let (output_sender, output_receiver) = std::sync::mpsc::channel::<()>();
    let (gpu_ready_sender, gpu_ready_receiver) = std::sync::mpsc::channel::<()>();
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
                Err(e) => debug!("Could not activate sending to broadcast: {}", e),
            }
            match socket.set_nonblocking(true) {
                Ok(_) => debug!("Activated non-blocking mode"),
                Err(e) => debug!("Could not activate non-blocking mode: {}", e),
            };

            let extract_output = ExtractOutput::get();

            for _ in output_receiver.iter() {
                let packages: Vec<(SocketAddr, Vec<u8>)> = {
                    let output_data = extract_output.poll_output_buffer();
                    gpu_ready_sender.send(()).ok();

                    let mut outputs = extract_output.outputs.lock();

                    extract_output
                        .universes
                        .lock()
                        .iter()
                        .take(UNIVERSES as usize)
                        .zip(output_data.chunks(UNIVERSE_BUFFER_SIZE as usize))
                        .filter_map(|(universe, data)| {
                            let universe_output = outputs.universe_output(*universe);

                            match universe_output {
                                UniverseOutput::Artnet { ip, universe } => {
                                    log::debug!("Preparing artnet command for universe {universe}");
                                    let output = artnet_protocol::Output {
                                        data: artnet_protocol::PaddedData::from(data.to_vec()),
                                        port_address: artnet_protocol::PortAddress::try_from(
                                            *universe,
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
                                UniverseOutput::WledDRGB { ip, port } => {
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
                                UniverseOutput::WledDNRGB { ip, port, start } => {
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
                    log::debug!("Sending package to {addr}");
                    log::trace!("Package data: {data:02x?}");
                    if let Err(err) = socket.send_to(&data, addr) {
                        error!("Could not send data: {:?}", err)
                    };
                }
            }
        })
        .context("Could not spawn artnet thread")?;

    Ok((output_sender, gpu_ready_receiver))
}
