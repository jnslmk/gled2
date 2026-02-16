use artnet_protocol::{ArtCommand, PaddedData, PollReply, PortAddress};
use chrono::{DateTime, Utc};
use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::mutex::Mutex;
use log::{debug, trace, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, UdpSocket},
    sync::Arc,
    thread,
    time::Duration,
};

use crate::{
    pipeline::{constants::UNIVERSE_BUFFER_SIZE, output_sender::OutputPackage},
    storage::asset::output_device::routing::OutputRouting,
};
use crate::input::external_control::ArtnetControlConfig;

static ARTNET_PORT: u16 = 6454;
pub static ARTNET_SOCKET: Lazy<Arc<UdpSocket>> = Lazy::new(|| {
    Arc::new(UdpSocket::bind(("0.0.0.0", ARTNET_PORT)).expect("Could not bind on artnet port"))
});

#[derive(Debug)]
pub struct ArtnetEvent {
    pub channel: u16,
    pub value: u8,
}

pub static ARTNET_CONFIG: Lazy<Mutex<ArtnetConfig>> = Lazy::new(|| Mutex::new(Default::default()));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ArtnetConfig {
    pub active: bool,
    pub universe: u16,
    pub bind_ip: Ipv4Addr,
    #[serde(deserialize_with = "crate::storage::serde::deserialize_u16_index_btreemap")]
    pub bridge: BTreeMap<u16, Bridge>,
    pub artnet_control_config: ArtnetControlConfig,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bridge {
    pub last_data_at: DateTime<Utc>,
    pub updates: usize,
    pub output_routing: OutputRouting,
    pub htp: bool,
}

impl Default for ArtnetConfig {
    fn default() -> Self {
        Self {
            active: false,
            universe: 18,
            bind_ip: Ipv4Addr::LOCALHOST,
            bridge: Default::default(),
            artnet_control_config: ArtnetControlConfig::default(),
        }
    }
}

impl ArtnetConfig {
    fn port_address(&self) -> PortAddress {
        PortAddress::try_from(self.universe).unwrap_or_else(|_| PortAddress::from(0))
    }
}

pub fn start_thread(
    output_package_sender: Sender<OutputPackage>,
) -> (Receiver<ArtnetEvent>, Receiver<PaddedData>) {
    let (bridge_sender, bridge_receiver) = unbounded();
    let (control_sender, control_receiver) = unbounded();

    trace!("Opening udp sockets on artnet port");

    let bridge_sender = bridge_sender.clone();
    let control_sender = control_sender.clone();
    thread::Builder::new()
        .name("gled:artnet:rx".to_string())
        .spawn(move || {
            #[cfg(feature = "profiling")]
            profiling::register_thread!("artnet:rx");

            loop {
                if !ARTNET_CONFIG.lock().active {
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }

                trace!("Starting artnet receiving");
                trace!("Setting broadcast");
                ARTNET_SOCKET
                    .set_broadcast(true)
                    .expect("Could not set broadcast");
                let mut buf = [0; 4096];
                loop {
                    if !ARTNET_CONFIG.lock().active {
                        trace!("Stopping artnet reaceiving");
                        break;
                    }

                    trace!("Receiving artnet package");
                    let Ok((size, src)) = ARTNET_SOCKET.recv_from(&mut buf) else {
                        debug!("Could not receive on artnet");
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    };
                    trace!("received on artnet");

                    crate::network_stats::add_incoming_bytes(size);

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
                                    bytes[..18].clone_from_slice("Gled2 Artnet Input".as_bytes());
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
                                bind_ip: ARTNET_CONFIG.lock().bind_ip.octets(),
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

                            crate::network_stats::add_outgoing_bytes(data.len());
                            match ARTNET_SOCKET.send_to(&data, src) {
                                Err(err) => {
                                    warn!("Could not send PollReply: {err:?}");
                                }
                                Ok(count) => {
                                    crate::network_stats::add_outgoing_bytes(count);
                                }
                            }

                            continue;
                        }
                        Ok(artnet) => {
                            debug!("Unhandeled ArtCommand: {artnet:?}");
                            continue;
                        }
                    };
                    trace!("parsed artnet");

                    let mut config = ARTNET_CONFIG.lock();

                    match output.port_address {
                        x if x == config.port_address() => {
                            trace!("artnet data on input universe");
                            let data = output.data.as_ref();
                            data.iter().enumerate().for_each(|(channel, value)|
                                bridge_sender
                                .send(ArtnetEvent {
                                    channel: channel as u16,
                                    value: *value,
                                })
                                .expect("Could not send event")
                            );
                        }
                        x if x == config.artnet_control_config.universe.try_into().unwrap() => {
                            trace!("artnet data on control universe");
                            control_sender
                                .send(output.data)
                                .expect("Could not send event");
                        }
                        _ => {
                            trace!("artnet data on another universe");

                            let bridge =
                                config.bridge.entry(output.port_address.into()).or_default();
                            bridge.last_data_at = Utc::now();
                            bridge.updates += 1;

                            if let Some(recipient) = bridge.output_routing.recipient() {
                                trace!("artnet bridge has output routing");

                                let mut data = [0u8; UNIVERSE_BUFFER_SIZE as usize];
                                data.copy_from_slice(output.data.as_ref());

                                output_package_sender
                                    .send(OutputPackage::ArtnetInput {
                                        recipient,
                                        data,
                                        htp: bridge.htp,
                                    })
                                    .ok();
                            }
                        }
                    }
                }
            }
        })
        .expect("Could not spawn artnet receive thread for {addr}");

    (bridge_receiver, control_receiver)
}
