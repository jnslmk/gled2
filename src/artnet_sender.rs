//! Send data via Art-Net udp protocol.
use anyhow::{Context, Result};
use artnet_protocol::ArtCommand;
use log::{error, info};
use std::{
    net::{ToSocketAddrs, UdpSocket},
    sync::{mpsc::Sender, RwLock},
    thread,
};

pub type ArtnetSender = Sender<ArtCommand>;

static ARTNET_HOST: RwLock<String> = RwLock::new(String::new());

pub fn set_artnet_host(artnet_host: String) {
    *ARTNET_HOST.write().expect("ARTNET_HOST is poisoned") = artnet_host;
}

/// Start artnet thread
pub fn start() -> Result<ArtnetSender> {
    info!("Spawning artnet thread");
    let (sender, receiver) = std::sync::mpsc::channel::<ArtCommand>();

    thread::Builder::new()
        .name("gled:artnet:tx".to_owned())
        .spawn(move || {
            let socket = UdpSocket::bind(("0.0.0.0", 6000)).unwrap();
            match socket.set_broadcast(true) {
                Ok(_) => info!("Activated sending to broadcast"),
                Err(e) => info!("Could not activate sending to broadcast: {}", e),
            }
            match socket.set_nonblocking(true) {
                Ok(_) => info!("Activated non-blocking mode"),
                Err(e) => info!("Could not activate non-blocking mode: {}", e),
            };

            for command in receiver.iter() {
                let Ok(bytes) = command
                    .write_to_buffer()
                    .map_err(|e| error!("Could not convert command into buffer: {:?}", e))
                    else {
                        continue;
                    };

                let Some(addr) = ARTNET_HOST
                    .read()
                    .ok()
                    .and_then(|host| (host.as_str(), 6454).to_socket_addrs().ok())
                    .and_then(|mut addr| addr.next()) else {
                        continue;
                    };

                log::info!("Sending artnet command");
                if let Err(err) = socket.send_to(&bytes, addr) {
                    error!("Could not send data: {:?}", err)
                };
            }
        })
        .context("Could not spawn artnet thread")?;

    Ok(sender)
}
