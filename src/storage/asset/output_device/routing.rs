use crate::storage::{AssetId, OutputDevice};
use egui::ahash::HashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct OutputRouting {
    pub device: Option<AssetId<OutputDevice>>,
    pub universe: Option<u16>,
}
