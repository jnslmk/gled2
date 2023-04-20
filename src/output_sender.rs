//! Send data via Art-Net udp protocol.
use anyhow::{Context, Result};
use artnet_protocol::{ArtCommand, Output, PaddedData, PortAddress};
use log::{debug, error};
use std::{
    net::{IpAddr, Ipv4Addr, ToSocketAddrs, UdpSocket},
    sync::{
        mpsc::{Receiver, Sender},
        RwLock,
    },
    thread,
};

use crate::{
    constants::{UNIVERSES, UNIVERSE_BUFFER_SIZE},
    extract_output::ExtractOutput,
};
pub type OutputSender = Sender<()>;
pub type GpuReadyReceiver = Receiver<()>;

/// Start output thread
pub fn start(mut extract_output: ExtractOutput) -> Result<(OutputSender, GpuReadyReceiver)> {
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

            for _ in output_receiver.iter() {
                let output_data = extract_output.poll_output_buffer();

                let commands: Vec<_> = extract_output
                    .universes()
                    .iter()
                    .take(UNIVERSES as usize)
                    .zip(output_data.chunks(UNIVERSE_BUFFER_SIZE as usize))
                    .filter_map(|(universe, data)| {
                        log::debug!("Preparing artnet command for universe {universe}");
                        let output = Output {
                            data: PaddedData::from(data.to_vec()),
                            port_address: PortAddress::try_from(*universe).ok()?,
                            ..Default::default()
                        };

                        Some(ArtCommand::Output(output))
                    })
                    .collect();

                gpu_ready_sender.send(()).ok();

                for command in commands {
                    let Ok(bytes) = command
                    .write_to_buffer()
                    .map_err(|e| error!("Could not convert command into buffer: {:?}", e))
                    else {
                        continue;
                    };

                    /* TODO: Support outputs
                    let Some(addr) = ARTNET_IP
                    .read()
                    .ok()
                    .and_then(|ip| (*ip, 6454).to_socket_addrs().ok())
                    .and_then(|mut addr| addr.next()) else {
                        continue;
                    };

                    log::debug!("Sending artnet command to {addr}");
                    log::trace!("Artnet data: {bytes:02x?}");
                    if let Err(err) = socket.send_to(&bytes, addr) {
                        error!("Could not send data: {:?}", err)
                    };
                    */
                }
            }
        })
        .context("Could not spawn artnet thread")?;

    Ok((output_sender, gpu_ready_receiver))
}
