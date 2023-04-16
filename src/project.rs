use crate::{app::Svg, get_pipeline, pipeline::Pipeline};
use log::{error, info};
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
        let pipeline = pipeline.clone();
        Self { svg, pipeline }
    }

    pub fn load(path: Option<&Path>) -> Self {
        let project = path
            .and_then(|path| {
                std::fs::read(path)
                    .map_err(|err| error!("Could not read project file: {err:?}"))
                    .ok()
            })
            .and_then(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|err| error!("Could not parse project file: {err:?}"))
                    .ok()
            });
        match project {
            None => {
                info!("Loaded demo project");
                Project {
                    pipeline: Pipeline::demo(),
                    svg: None,
                }
            }
            Some(project) => {
                info!(
                    "Loaded project file at {}",
                    path.expect("must be set at this point").display()
                );

                project
            }
        }
    }

    pub fn store(&self, path: &Path) {
        let contents = match serde_json::to_string_pretty(&self) {
            Ok(contents) => contents,
            Err(err) => {
                error!("Could not serialize project: {err:?}");
                return;
            }
        };
        if let Err(err) = std::fs::write(path, contents) {
            error!(
                "Could not write project file to {}: {err:?}",
                path.display()
            );
        }

        info!("Saved project file at {}", path.display());
    }
}
