use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Output {
    Artnet { ip: IpAddr },
    WledDRGB { ip: IpAddr, port: u16 },
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
    Default,
    Artnet { ip: IpAddr, universe: u16 },
    WledDRGB { ip: IpAddr, port: u16 },
}
