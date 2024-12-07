mod state;

use crate::{
    app::positions,
    group::Groups,
    storage::{
        Animation, AnimationConfig, Asset, AssetId, Palette, RangeDegrees, RangePercentage,
        StaticOrCurve,
    },
    OUTPUT_BUFFER,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{CommandEncoder, Queue};

pub use state::EffectState;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Effect {
    pub color_shift: StaticOrCurve<RangeDegrees>,
    pub opacity: StaticOrCurve<RangePercentage>,
    pub beat_progression: StaticOrCurve<RangePercentage>,
    pub beat_progression_offset: StaticOrCurve<RangePercentage>,
    /// In 2^n of bpm
    pub speed_exponent: i32,
    pub group_index: usize,
    pub animation: Option<AssetId<Animation>>,
    pub animation_config: AnimationConfig,

    /// Overwrite the animation with this one, must be only used in the code editor/preview
    #[serde(skip)]
    pub animation_overwrite: Option<Arc<Asset<Animation>>>,
}

impl Default for Effect {
    fn default() -> Self {
        Self {
            color_shift: StaticOrCurve::new_static(0.0),
            opacity: StaticOrCurve::new_static(1.0),
            beat_progression: StaticOrCurve::new_curve([
                0x2E, 0x07, 0x5C, 0xCB, 0x90, 0xC4, 0x47, 0x4B, 0x8D, 0x16, 0xC6, 0x70, 0x40, 0x54,
                0x99, 0xA6,
            ]),
            beat_progression_offset: StaticOrCurve::new_static(0.0),
            speed_exponent: Default::default(),
            group_index: Default::default(),
            animation: Default::default(),
            animation_config: Default::default(),
            animation_overwrite: Default::default(),
        }
    }
}

impl Effect {
    pub fn shader_code_complete(&self) -> String {
        self.animation_overwrite
            .clone()
            .or_else(|| self.animation.and_then(Asset::get))
            .map(|animation| animation.data.shader_code_complete())
            .unwrap_or_else(|| {
                format!(
                    "{}\n\n{}",
                    include_str!("shaders/common.wgsl"),
                    include_str!("shaders/red.wgsl")
                )
            })
    }

    pub fn set_output_mix_buffers(&self, state: &mut EffectState) {
        let other = state.texture_to_output.output_buffer();
        state.output_mix.set_buffers(&OUTPUT_BUFFER, other);
    }

    pub fn prepare(
        &self,
        effect_state: &mut EffectState,
        queue: &Queue,
        palette: Option<Arc<Asset<Palette>>>,
        groups: &Groups,
        main_opacity: f32,
    ) {
        let beat_progression = effect_state.beat_progression
            + self
                .beat_progression_offset
                .value(effect_state.beat_progression);
        effect_state.beat_progression = self.beat_progression.value(beat_progression);
        effect_state.opacity = self.opacity.value(beat_progression) * main_opacity;
        effect_state.color_shift = self.color_shift.value(beat_progression);
        effect_state.speed_exponent = self.speed_exponent;
        effect_state.animation_config = self.animation_config.clone();

        let group = groups.get(self.group_index);
        if effect_state.sent_group.as_ref() != group {
            let positions = group.map(positions);
            effect_state
                .texture_to_output
                .set_positions(queue, positions.unwrap_or_default());
            effect_state.sent_group = group.cloned();
        }

        effect_state
            .renderer
            .set_buffers(queue, effect_state, palette);
    }

    pub fn render(
        &self,
        effect_state: &mut EffectState,
        encoder: &mut CommandEncoder,
        send_output: bool,
    ) {
        effect_state.renderer.render(encoder);
        if send_output {
            effect_state.texture_to_output.run(encoder);
            effect_state.output_mix.run(encoder);
        }
    }
}
