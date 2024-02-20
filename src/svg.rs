//! Read parameters like fixture and render groups for each led from svg files

mod led;
mod measurement_point;
mod parameter;
mod render;

use anyhow::{Context, Result};
use log::{debug, info};
use std::collections::{HashMap, HashSet};
use svgdom::{Document, ElementId, FilterSvg, Node};
use usvg::Tree;

pub use led::Led;
pub use measurement_point::{MeasurementPoints, Universes};
pub use parameter::Parameter;

pub struct ParsedSvg {
    pub parameters: HashMap<String, Parameter>,
    pub tree: Tree,
}

impl std::fmt::Debug for ParsedSvg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Svg")
            .field("parameters", &self.parameters)
            .finish()
    }
}

impl ParsedSvg {
    /// Open and parse svg files and determine parameters
    pub fn parse(svg_contents: &str) -> Result<Self> {
        debug!("Parsing svg");
        let doc = Document::from_str(svg_contents).context("Could not parse svg file")?;

        let mut parameters = HashMap::new();
        traverse_node(&mut parameters, &doc.root(), 0, 0, &HashSet::new());

        let tree = Tree::from_str(
            svg_contents,
            &Default::default(),
            &usvg::fontdb::Database::default(),
        )
        .unwrap();
        debug!(
            "Done parsing svg. Found {} parameter sets",
            parameters.len()
        );

        Ok(Self { parameters, tree })
    }
}

/// Recursivly traverse nodes and read parameters
fn traverse_node(
    parameters: &mut HashMap<String, Parameter>,
    node: &Node,
    start: u32,
    universe: u16,
    parents: &HashSet<String>,
) {
    node.children()
        .svg()
        .filter_map(|(id, child)| {
            if id == ElementId::Desc {
                None
            } else {
                Some(child)
            }
        })
        .for_each(|node| {
            let mut start = start;
            let mut universe = universe;

            let mut parents = parents.clone();
            if let Some(mut parameter) = node
                .children()
                .svg()
                .filter_map(|(id, child)| {
                    if id == ElementId::Desc {
                        Some(child)
                    } else {
                        None
                    }
                })
                .next()
                .and_then(|child| child.children().next())
                .and_then(|child| {
                    serde_json::from_str::<Parameter>(&child.text())
                        .map_err(|e| info!("Could not parse parameter of {}: {}", node.id(), e))
                        .ok()
                })
            {
                parameter.start += start;
                parameter.universe += universe;

                start = parameter.start;
                universe = parameter.universe;

                parents.insert(node.id().clone());
                let leds = parameter.leds();
                parents.iter().for_each(|parent_id| {
                    if let Some(parameter) = parameters.get_mut(parent_id) {
                        parameter.leds.extend(leds.clone())
                    }
                });
                parameter.leds.extend(leds);
                parameters.insert(node.id().clone(), parameter);
            }

            traverse_node(parameters, &node, start, universe, &parents);
        });
}
