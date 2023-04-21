use eframe::epaint::ahash::HashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Outputs {
    default_output: Output,
    universe_outputs: HashMap<u16, UniverseOutput>,
}

impl Outputs {
    pub fn default_output_mut(&mut self) -> &mut Output {
        &mut self.default_output
    }

    pub fn default_output(&self) -> &Output {
        &self.default_output
    }

    pub fn universe_output_mut(&mut self, universe: u16) -> &mut UniverseOutput {
        self.universe_outputs
            .entry(universe)
            .or_insert_with(|| UniverseOutput::Default)
    }

    pub fn universe_output(&self, universe: u16) -> UniverseOutput {
        self.universe_outputs
            .get(&universe)
            .cloned()
            .unwrap_or_default()
    }

    pub fn universe_output_normalized(&self, universe: u16) -> UniverseOutput {
        match self.universe_outputs.get(&universe) {
            Some(UniverseOutput::Default) | None => match self.default_output {
                Output::Artnet { ip } => UniverseOutput::Artnet { ip, universe },
                Output::WledDRGB { ip, port } => UniverseOutput::WledDRGB { ip, port },
                Output::WledDNRGB { ip, port, start } => {
                    UniverseOutput::WledDNRGB { ip, port, start }
                }
            },
            Some(universe_output) => universe_output.clone(),
        }
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum UniverseOutput {
    #[default]
    Default,
    Artnet {
        ip: IpAddr,
        universe: u16,
    },
    WledDRGB {
        ip: IpAddr,
        port: u16,
    },
    WledDNRGB {
        ip: IpAddr,
        port: u16,
        start: u16,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OutputKind {
    Default,
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
            UniverseOutput::Default => OutputKind::Default,
            UniverseOutput::Artnet { .. } => OutputKind::Artnet,
            UniverseOutput::WledDRGB { .. } => OutputKind::WledDRGB,
            UniverseOutput::WledDNRGB { .. } => OutputKind::WledDNRGB,
        }
    }
}
