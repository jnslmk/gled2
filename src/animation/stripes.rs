use super::{
    config::{CommonConfig, Config},
    renderer::AnimationRenderer,
    Animation,
};
use wgpu::Device;

pub struct Stripes {
    renderer: AnimationRenderer,
    config: StripesConfig,
}

impl Stripes {
    pub fn new(device: &Device, palette: &super::ColorPalette, config: StripesConfig) -> Self {
        let fragment_shader = include_str!("../shaders/stripes.wgsl");
        let renderer = AnimationRenderer::new(device, fragment_shader, palette, &(&config).into());

        Self { renderer, config }
    }

    pub fn renderer(&self) -> &AnimationRenderer {
        &self.renderer
    }

    pub fn config(&self) -> &StripesConfig {
        &self.config
    }
}

// TODO: Macro
impl From<Stripes> for Animation {
    fn from(line_sweep: Stripes) -> Self {
        Self::Stripes(line_sweep)
    }
}

pub struct StripesConfig {
    pub common: CommonConfig,
    pub thickness: f32,
    pub count: i32,
}

impl Default for StripesConfig {
    fn default() -> Self {
        Self {
            common: Default::default(),
            thickness: 0.1,
            count: 5,
        }
    }
}

impl From<&StripesConfig> for Config {
    fn from(config: &StripesConfig) -> Self {
        Config {
            common: config.common,
            thickness: config.thickness,
            count: config.count,
            ..Default::default()
        }
    }
}
