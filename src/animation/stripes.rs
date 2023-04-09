use super::{
    config::{CommonConfig, Config},
    renderer::AnimationRenderer,
    Animation,
};

#[derive(Default)]
pub struct Stripes {
    renderer: Option<AnimationRenderer>,
    config: StripesConfig,
}

impl Stripes {
    pub fn new(config: StripesConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn renderer(&mut self) -> &AnimationRenderer {
        self.renderer.get_or_insert_with(|| {
            let fragment_shader = include_str!("../shaders/stripes.wgsl");
            AnimationRenderer::new(fragment_shader, &(&self.config).into())
        })
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
