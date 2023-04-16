//! Send data via Art-Net udp protocol.
use anyhow::{Context, Result};
use log::{debug, error};
use std::{
    net::{IpAddr, Ipv4Addr, ToSocketAddrs, UdpSocket},
    sync::{
        mpsc::{Receiver, Sender},
        RwLock,
    },
    thread,
};

use crate::extract_artnet::ExtractArtnet;
pub type ArtnetSender = Sender<()>;
pub type GpuReadyReceiver = Receiver<()>;

static ARTNET_IP: RwLock<IpAddr> = RwLock::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

pub fn set_artnet_ip(artnet_ip: IpAddr) {
    *ARTNET_IP.write().expect("ARTNET_HOST is poisoned") = artnet_ip;
}

/// Start artnet thread
pub fn start(mut extract_artnet: ExtractArtnet) -> Result<(ArtnetSender, GpuReadyReceiver)> {
    debug!("Spawning artnet thread");
    let (artnet_sender, artnet_receiver) = std::sync::mpsc::channel::<()>();
    let (gpu_ready_sender, gpu_ready_receiver) = std::sync::mpsc::channel::<()>();
    gpu_ready_sender.send(()).ok();

    thread::Builder::new()
        .name("gled:artnet:tx".to_owned())
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

            for _ in artnet_receiver.iter() {
                let commands = extract_artnet.poll_artnet_buffer();
                gpu_ready_sender.send(()).ok();

                for command in commands {
                    let Ok(bytes) = command
                    .write_to_buffer()
                    .map_err(|e| error!("Could not convert command into buffer: {:?}", e))
                    else {
                        continue;
                    };

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
                }
            }
        })
        .context("Could not spawn artnet thread")?;

    Ok((artnet_sender, gpu_ready_receiver))
}
