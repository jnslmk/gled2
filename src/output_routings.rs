use crate::storage::{AssetId, OutputDevice};
use egui::ahash::HashMap;
use serde::{Deserialize, Serialize};

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
    pub device: Option<AssetId<OutputDevice>>,
    pub universe: Option<u16>,
}
