use crate::{app::Svg, get_pipeline, pipeline::Pipeline};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Default)]
pub struct Project {
    pub pipeline: Pipeline,
    pub svg: Option<Svg>,
}

impl Project {
    pub fn from_svg(svg: Option<Svg>) -> Self {
        get_pipeline!(pipeline);
        Self {
            svg,
            pipeline: pipeline.clone(),
        }
    }

    pub fn load(_path: Option<&Path>) -> Self {
        //TODO: Load from disk
        Self::default()
    }
}
