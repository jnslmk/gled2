use eframe::epaint::ahash::HashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use strum::{EnumIter, IntoStaticStr};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum OutputDevice {
    Artnet {
        name: String,
        ip: IpAddr,
        universes: Vec<u16>,
    },
    WledDRGB {
        name: String,
        ip: IpAddr,
        port: u16,
    },
    WledDNRGB {
        name: String,
        ip: IpAddr,
        port: u16,
        start: u16,
    },
}

impl OutputDevice {
    pub fn name(&self) -> &str {
        match self {
            OutputDevice::Artnet { name, .. }
            | OutputDevice::WledDRGB { name, .. }
            | OutputDevice::WledDNRGB { name, .. } => name,
        }
    }

    pub fn set_name(&mut self, name: String) {
        match self {
            OutputDevice::Artnet { name: n, .. }
            | OutputDevice::WledDRGB { name: n, .. }
            | OutputDevice::WledDNRGB { name: n, .. } => *n = name,
        }
    }

    pub fn ip(&self) -> IpAddr {
        match self {
            OutputDevice::Artnet { ip, .. }
            | OutputDevice::WledDRGB { ip, .. }
            | OutputDevice::WledDNRGB { ip, .. } => *ip,
        }
    }

    pub fn set_ip(&mut self, ip: IpAddr) {
        match self {
            OutputDevice::Artnet { ip: i, .. }
            | OutputDevice::WledDRGB { ip: i, .. }
            | OutputDevice::WledDNRGB { ip: i, .. } => *i = ip,
        }
    }

    pub fn universes(&self) -> &[u16] {
        match self {
            OutputDevice::Artnet { universes, .. } => universes,
            OutputDevice::WledDRGB { .. } | OutputDevice::WledDNRGB { .. } => &[],
        }
    }

    pub fn kind(&self) -> OutputDeviceKind {
        match self {
            OutputDevice::Artnet { .. } => OutputDeviceKind::Artnet,
            OutputDevice::WledDRGB { .. } => OutputDeviceKind::WledDRGB,
            OutputDevice::WledDNRGB { .. } => OutputDeviceKind::WledDNRGB,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, IntoStaticStr, PartialEq, Eq, EnumIter)]
pub enum OutputDeviceKind {
    Artnet,
    WledDRGB,
    WledDNRGB,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OutputRoutings {
    routings: HashMap<u16, OutputRouting>,
}

impl OutputRoutings {
    pub fn universe_output_routing(&mut self, universe: u16) -> &mut OutputRouting {
        self.routings.entry(universe).or_default()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OutputRouting {
    pub device: Option<Uuid>,
    pub universe: Option<u16>,
}
