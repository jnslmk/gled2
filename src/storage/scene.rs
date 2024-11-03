use super::asset::AssetTrait;
use crate::effect::Effect;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct Scene {
    primary_effects: Vec<Effect>,
    secondary_effects: Vec<Effect>,
}

impl AssetTrait for Scene {
    const DIR_NAME: &'static str = "scenes";
    const NAME: &'static str = "Scene";
}
