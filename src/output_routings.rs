use crate::storage::{AssetId, OutputDevice};
use egui::ahash::HashMap;
use serde::{Deserialize, Serialize};

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
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OutputRouting {
    pub device: Option<AssetId<OutputDevice>>,
    pub universe: Option<u16>,
}
