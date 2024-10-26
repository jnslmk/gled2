use eframe::epaint::ahash::HashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Outputs {
    universe_outputs: HashMap<u16, UniverseOutput>,
}

impl Outputs {
    pub fn universe_output(&mut self, universe: u16) -> &mut UniverseOutput {
        self.universe_outputs.entry(universe).or_default()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Output {
    Artnet { ip: IpAddr },
    WledDRGB { ip: IpAddr, port: u16 },
    WledDNRGB { ip: IpAddr, port: u16, start: u16 },
}

impl Default for Output {
    fn default() -> Self {
        Self::Artnet {
            ip: "127.0.0.1".parse().expect("Could not parse 127.0.0.1"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UniverseOutput {
    Artnet { ip: IpAddr, universe: u16 },
    WledDRGB { ip: IpAddr, port: u16 },
    WledDNRGB { ip: IpAddr, port: u16, start: u16 },
}

impl Default for UniverseOutput {
    fn default() -> Self {
        Self::Artnet {
            ip: [127, 0, 0, 1].into(),
            universe: 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OutputKind {
    Artnet,
    WledDRGB,
    WledDNRGB,
}

impl Output {
    pub fn kind(&self) -> OutputKind {
        match self {
            Output::Artnet { .. } => OutputKind::Artnet,
            Output::WledDRGB { .. } => OutputKind::WledDRGB,
            Output::WledDNRGB { .. } => OutputKind::WledDNRGB,
        }
    }
}

impl UniverseOutput {
    pub fn kind(&self) -> OutputKind {
        match self {
            UniverseOutput::Artnet { .. } => OutputKind::Artnet,
            UniverseOutput::WledDRGB { .. } => OutputKind::WledDRGB,
            UniverseOutput::WledDNRGB { .. } => OutputKind::WledDNRGB,
        }
    }
}
