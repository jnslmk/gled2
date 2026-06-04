use crate::input::external_control::ArtnetControlConfig;
use crate::{
    pipeline::{constants::UNIVERSE_BUFFER_SIZE, output_sender::OutputPackage},
    storage::{asset::output_device::routing::OutputRouting, collections::Collections},
};
use artnet_protocol::{ArtCommand, PollReply, PortAddress};
use chrono::{DateTime, Utc};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, unbounded};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::ops::Shl;
use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, UdpSocket},
    sync::Arc,
    thread,
    time::Duration,
};
use sysinfo::Networks;
use tracing::{debug, trace, warn};

static ARTNET_PORT: u16 = 6454;
pub static ARTNET_SOCKET: Lazy<Arc<UdpSocket>> = Lazy::new(|| {
    let socket = UdpSocket::bind(("0.0.0.0", ARTNET_PORT)).expect("Could not bind on artnet port");
    // Enlarge the kernel send buffer. At hundreds of fps × dozens of universes
    // the default ~200 KB buffer overflows instantly, forcing every send into a
    // WouldBlock retry spin that throttled output throughput to ~20 Mbit.
    if let Err(e) = socket2::SockRef::from(&socket).set_send_buffer_size(8 * 1024 * 1024) {
        warn!("Could not enlarge artnet socket send buffer: {e}");
    }
    Arc::new(socket)
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
    pub universe: u16,
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
            universe: 18,
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
) -> (Receiver<ArtnetEvent>, Receiver<Vec<u8>>) {
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

            let mut collections = Collections::default();

            trace!("Starting artnet receiving");
            trace!("Setting broadcast");
            ARTNET_SOCKET
                .set_broadcast(true)
                .expect("Could not set broadcast");
            let mut buf = [0; 4096];
            loop {
                collections.update();

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

                        let networks = Networks::new_with_refreshed_list();
                        let bind_addr = &networks
                            .values()
                            .flat_map(|network| network.ip_networks().iter())
                            .find_map(|ip_network| match (ip_network.addr, src.ip()) {
                                (IpAddr::V4(interface_addr), IpAddr::V4(src_addr)) => {
                                    let netmask = u32::MAX.shl(32 - ip_network.prefix);
                                    ((interface_addr.to_bits() & netmask)
                                        == (src_addr.to_bits() & netmask))
                                        .then_some(interface_addr)
                                }
                                _ => None,
                            });

                        if let Some(bind_addr) = bind_addr {
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
                                bind_ip: bind_addr.octets(),
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
                // dispatch trigger packages
                if output.port_address == config.port_address() {
                    trace!("artnet data on input universe");
                    let data = output.data.as_ref();
                    data.iter().enumerate().for_each(|(channel, value)| {
                        bridge_sender
                            .send(ArtnetEvent {
                                channel: channel as u16,
                                value: *value,
                            })
                            .expect("Could not send event")
                    });
                }
                // dispatch control packages
                if output.port_address
                    == config
                        .artnet_control_config
                        .universe
                        .try_into()
                        .expect("control universe value out of valid PortAddress range")
                {
                    trace!("artnet data on control universe");
                    control_sender
                        .send(output.data.as_ref().to_vec())
                        .expect("Could not send event");
                }
                // dispatch all artnet data to the bridge
                let bridge = config.bridge.entry(output.port_address.into()).or_default();
                bridge.last_data_at = Utc::now();
                bridge.updates += 1;

                if let Some(recipient) = bridge.output_routing.recipient(&collections) {
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
        })
        .expect("Could not spawn artnet receive thread");

    (bridge_receiver, control_receiver)
}
