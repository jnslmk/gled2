pub mod enttec_usb_pro;
pub mod routing;

use super::AssetTrait;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, ToSocketAddrs};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum OutputDevice {
    Artnet { ip: IpAddr, universes: Vec<u16> },
    EnttecDmxUsbPro { serial_number: String },
}

impl Default for OutputDevice {
    fn default() -> Self {
        Self::Artnet {
            ip: [127, 0, 0, 1].into(),
            universes: vec![],
        }
    }
}

impl AssetTrait for OutputDevice {
    const DIR_NAME: &'static str = "output_devices";
    const NAME: &'static str = "Output Device";
    const SHOW_NAME_IF_SELECTED: bool = true;
}

impl OutputDevice {
    pub fn ip(&self) -> Option<IpAddr> {
        match self {
            OutputDevice::Artnet { ip, .. } => Some(*ip),
            _ => None,
        }
    }

    pub fn set_ip(&mut self, ip: IpAddr) {
        if let OutputDevice::Artnet { ip: i, .. } = self {
            *i = ip
        }
    }

    pub fn universes(&self) -> &[u16] {
        match self {
            OutputDevice::Artnet { universes, .. } => universes,
            _ => &[],
        }
    }

    pub fn set_univeres(&mut self, universes: Vec<u16>) {
        if let OutputDevice::Artnet { universes: u, .. } = self {
            *u = universes
        }
    }

    pub fn prepare_package(
        &self,
        universe: Option<u16>,
        data: &[u8],
    ) -> Option<(std::net::SocketAddr, Vec<u8>)> {
        match self {
            OutputDevice::Artnet { ip, universes, .. } => {
                let universe = universe?;
                if !universes.contains(&universe) {
                    log::warn!("Universe which is not configured: {universe}");
                    return None;
                }

                log::debug!("Preparing artnet command for universe {universe}");
                let output = artnet_protocol::Output {
                    data: artnet_protocol::PaddedData::from(data.to_vec()),
                    port_address: artnet_protocol::PortAddress::try_from(universe).ok()?,
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
        }
    }
}
