pub mod enttec_usb_pro;
pub mod routing;

use crate::pipeline::output_sender::Recipient;

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

    pub fn get_recipient(&self, universe: Option<u16>) -> Option<Recipient> {
        match self {
            OutputDevice::Artnet { ip, universes, .. } => {
                let universe = universe?;
                if !universes.contains(&universe) {
                    log::warn!("Universe which is not configured: {universe}");
                    return None;
                }
                (*ip, 6454)
                    .to_socket_addrs()
                    .ok()
                    .and_then(|mut addrs| addrs.next())
                    .map(|addr| Recipient::Artnet { addr, universe })
            }
            OutputDevice::EnttecDmxUsbPro { serial_number } => Some(Recipient::EnttecDmxUsbPro {
                serial_number: serial_number.clone(),
            }),
        }
    }
}
