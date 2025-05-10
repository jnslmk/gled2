use artnet_protocol::{ArtCommand, PollReply, PortAddress};
use egui::mutex::Mutex;
use log::{debug, info, trace, warn};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    net::{Ipv4Addr, UdpSocket},
    sync::mpsc::{Receiver, Sender},
    thread,
    time::Duration,
};

static ARTNET_PORT: u16 = 6454;

#[derive(Debug)]
pub struct ArtnetEvent {
    pub channel: u8,
    pub value: u8,
}

pub static ARTNET_CONFIG: Lazy<Mutex<ArtnetConfig>> = Lazy::new(|| Mutex::new(Default::default()));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ArtnetConfig {
    pub active: bool,
    pub universe: u16,
    pub start: u16,
    pub channels: u16,
}

impl Default for ArtnetConfig {
    fn default() -> Self {
        Self {
            active: false,
            universe: 18,
            start: 1,
            channels: 100,
        }
    }
}

impl ArtnetConfig {
    fn port_address(&self) -> PortAddress {
        PortAddress::try_from(self.universe).unwrap_or_else(|_| PortAddress::from(0))
    }
}

pub fn start_thread() -> Receiver<ArtnetEvent> {
    let (sender, receiver) = std::sync::mpsc::channel();

    info!("Spawning worker threads");
    threads(sender);

    receiver
}

fn threads(sender: Sender<ArtnetEvent>) {
    trace!("Opening udp sockets on artnet port");
    for addr in NetworkInterface::show()
        .expect("Could not find network interfaces")
        .into_iter()
        .flat_map(|interface| {
            interface
                .addr
                .into_iter()
                .filter(|addr| addr.ip().is_ipv4())
        })
    {
        let sender = sender.clone();
        thread::Builder::new()
            .name("gled:artnet:rx".to_string())
            .spawn(move || {
                loop {
                    if !ARTNET_CONFIG.lock().active {
                        thread::sleep(Duration::from_secs(1));
                        continue;
                    }

                    let ip = addr.ip();
                    trace!("Starting artnet receiving on {ip:?}");
                    let socket =
                        UdpSocket::bind((ip, ARTNET_PORT)).expect("Could not bind on artnet port");
                    trace!("Setting broadcast");
                    socket.set_broadcast(true).expect("Could not set broadcast");
                    let mut buf = [0; 4096];
                    loop {
                        if !ARTNET_CONFIG.lock().active {
                            trace!("Stopping artnet reaceiving");
                            break;
                        }

                        trace!("Receiving artnet package");
                        let Ok((size, src)) = socket.recv_from(&mut buf) else {
                            debug!("Could not receive on artnet");
                            std::thread::sleep(Duration::from_millis(10));
                            continue;
                        };
                        trace!("received on artnet");

                        let output = match ArtCommand::from_buffer(&buf[..size]) {
                            Err(err) => {
                                warn!("Could not parse artnet package: {err:?}");
                                continue;
                            }
                            Ok(ArtCommand::Output(output)) => output,
                            Ok(ArtCommand::Poll(_poll)) => {
                                debug!("Artnet poll from {src:?}");
                                let poll_reply = PollReply {
                                    address: match src.ip() {
                                        std::net::IpAddr::V4(ip) => ip,
                                        _ => Ipv4Addr::LOCALHOST,
                                    },
                                    port: ARTNET_PORT,
                                    version: [0, 0],
                                    port_address: [0, 0],
                                    oem: [0, 0],
                                    ubea_version: 0,
                                    status_1: 210,
                                    esta_code: 31344,
                                    short_name: {
                                        let mut bytes = [0; 18];
                                        bytes[..5].clone_from_slice("Gled2".as_bytes());
                                        bytes
                                    },
                                    long_name: {
                                        let mut bytes = [0; 64];
                                        bytes[..18]
                                            .clone_from_slice("Gled2 Artnet Input".as_bytes());
                                        bytes
                                    },
                                    node_report: [0; 64],
                                    num_ports: [0, 4],
                                    port_types: [192; 4],
                                    good_input: [0, 8, 8, 8],
                                    good_output: [0, 0, 0, 0],
                                    swin: [0; 4],
                                    swout: [0; 4],
                                    sw_video: 0,
                                    sw_macro: 0,
                                    sw_remote: 0,
                                    spare: [0; 3],
                                    style: 0,
                                    mac: [0; 6],
                                    bind_ip: match ip {
                                        std::net::IpAddr::V4(ip) => ip.octets(),
                                        _ => Ipv4Addr::LOCALHOST.octets(),
                                    },
                                    bind_index: 0,
                                    status_2: 0,
                                    filler: [0; 26],
                                };

                                let Ok(data) =
                                    ArtCommand::PollReply(Box::new(poll_reply)).write_to_buffer()
                                else {
                                    warn!("Could not create poll reply");
                                    continue;
                                };

                                if let Err(err) = socket.send_to(&data, src) {
                                    warn!("Could not send PollReply: {err:?}");
                                }

                                continue;
                            }
                            Ok(artnet) => {
                                debug!("Unhandeled ArtCommand: {artnet:?}");
                                continue;
                            }
                        };
                        trace!("parsed artnet");

                        let config = ARTNET_CONFIG.lock();
                        if output.port_address != config.port_address() {
                            debug!("Ignoring universe {:?}", output.port_address);
                            std::thread::sleep(Duration::from_millis(10));
                            continue;
                        }
                        trace!("data on correct universe");

                        let data = output.data.as_ref();

                        for i in 0..config.channels as usize {
                            let channel = config.start as usize + i - 1;
                            sender
                                .send(ArtnetEvent {
                                    channel: i as u8,
                                    value: data[channel],
                                })
                                .expect("Could not send event");
                        }
                    }
                }
            })
            .expect("Could not spawn artnet receive thread for {addr}");
    }
}
