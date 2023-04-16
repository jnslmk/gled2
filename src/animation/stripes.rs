use super::{
    config::{CommonConfig, Config},
    renderer::AnimationRenderer,
    Animation,
};
use egui::Ui;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Stripes {
    #[serde(skip)]
    pub renderer: Option<AnimationRenderer>,
    pub config: StripesConfig,
}

impl Clone for Stripes {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            ..Default::default()
        }
    }
}

impl Stripes {
    pub fn new(config: StripesConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn init_gpu(&mut self) {
        self.renderer.get_or_insert_with(|| {
            let fragment_shader = include_str!("../shaders/stripes.wgsl");
            AnimationRenderer::new(fragment_shader, &(&self.config).into())
        });
    }
}

impl From<Stripes> for Animation {
    fn from(line_sweep: Stripes) -> Self {
        Self::Stripes(line_sweep)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct StripesConfig {
    pub common: CommonConfig,
    pub thickness: f32,
    pub count: u32,
    pub orientation: Orientation,
}

impl StripesConfig {
    pub fn ui(&mut self, ui: &mut Ui) {
        self.common.ui(ui);
        ui.label("Todo: stripes");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.orientation, Orientation::Horizontal, "Horizontal");
            ui.radio_value(&mut self.orientation, Orientation::Vertical, "Vertical");
        });
    }
}

impl Default for StripesConfig {
    fn default() -> Self {
        Self {
            common: Default::default(),
            thickness: 0.1,
            count: 5,
            orientation: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl From<&StripesConfig> for Config {
    fn from(config: &StripesConfig) -> Self {
        Config {
            common: config.common,
            thickness: config.thickness,
            count: config.count,
            mode: match config.orientation {
                Orientation::Horizontal => 0,
                Orientation::Vertical => 1,
            },
            ..Default::default()
        }
    }
}
