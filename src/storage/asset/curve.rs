use super::AssetTrait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct Curve {
    //TODO
}

impl AssetTrait for Curve {
    const DIR_NAME: &'static str = "curves";
    const NAME: &'static str = "Curve";
}
