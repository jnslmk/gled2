//! A LED with all properties to send data to an actual lamp.
use super::color_channels::ColorChannels;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Led {
    pub universe: u16,
    /// Nth LED in the universe (Address is `num * 3`).
    pub num: usize,
    pub color_channels: ColorChannels,
}
