use crate::{
    pipeline::output_sender::Recipient,
    storage::{AssetId, OutputDevice, asset::Asset, collections::Collections},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::net::ToSocketAddrs;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct OutputRoutings {
    #[serde(deserialize_with = "deserialize_u16_map")]
    routings: HashMap<u16, OutputRouting>,
}

fn deserialize_u16_map<'de, D>(deserializer: D) -> Result<HashMap<u16, OutputRouting>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    HashMap::<String, OutputRouting>::deserialize(deserializer).map(|map| {
        map.into_iter()
            .filter_map(|(universe, routing)| {
                universe.parse().ok().map(|universe| (universe, routing))
            })
            .collect()
    })
}

impl OutputRoutings {
    pub fn get(&self, universe: &u16) -> Option<&OutputRouting> {
        self.routings.get(universe)
    }

    pub fn universe_output_routing(&mut self, universe: u16) -> &mut OutputRouting {
        self.routings.entry(universe).or_default()
    }

    pub fn remove_old(&mut self, universes: &BTreeSet<u16>) {
        self.routings
            .retain(|universe, _| universes.contains(universe));
    }

    pub fn output_universes_which_are_used_multiple_times(
        &self,
    ) -> BTreeSet<(AssetId<OutputDevice>, u16)> {
        let mut used = BTreeSet::new();
        let mut used_multiple_times = BTreeSet::new();
        for routing in self.routings.values() {
            if let (Some(device), Some(universe)) = (routing.device, routing.universe) {
                let key = (device, universe);
                if !used.insert(key) {
                    used_multiple_times.insert(key);
                }
            }
        }
        used_multiple_times
    }

    pub fn is_empty(&self) -> bool {
        self.routings.is_empty()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputRouting {
    pub device: Option<AssetId<OutputDevice>>,
    pub universe: Option<u16>,
}

impl OutputRouting {
    pub fn recipient(&self, collections: &Collections) -> Option<Recipient> {
        let device = self.device.and_then(|id| Asset::get(id, collections))?;

        match &device.data {
            OutputDevice::Artnet { ip, universes, .. } => {
                let universe = self.universe?;
                if !universes.contains(&universe) {
                    tracing::warn!("Universe which is not configured: {universe}");
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
