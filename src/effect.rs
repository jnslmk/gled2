mod state;

use crate::{
    animation::{Animation, AnimationConfig},
    app::positions,
    group::Groups,
    storage::{Asset, Palette, RangeDegrees, RangePercentage, StaticOrCurve},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{Buffer, CommandEncoder, Queue};

pub use state::EffectState;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Effect {
    pub color_shift: StaticOrCurve<RangeDegrees>,
    pub opacity: StaticOrCurve<RangePercentage>,
    pub beat_progression: StaticOrCurve<RangePercentage>,
    pub beat_progression_offset: StaticOrCurve<RangePercentage>,
    /// Whether it should use the primary or the secondary group
    pub use_secondary_group: bool,
    pub animation: Animation,
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
            use_secondary_group: Default::default(),
            animation: Default::default(),
        }
    }
}

impl PartialEq for Effect {
    fn eq(&self, other: &Self) -> bool {
        self.color_shift == other.color_shift
            && self.opacity == other.opacity
            && self.beat_progression_offset == other.beat_progression_offset
            && self.use_secondary_group == other.use_secondary_group
            && self.animation == other.animation
    }
}

impl Eq for Effect {}

impl Effect {
    pub fn new(animation: Animation) -> Self {
        Self {
            animation,
            ..Default::default()
        }
    }

    pub fn set_buffers(&self, state: &mut EffectState, main: &Buffer) {
        let other = state.texture_to_output.output_buffer();
        state.output_mix.set_buffers(main, other);
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

        let group = groups.get(self.use_secondary_group);
        if effect_state.sent_group.as_ref() != group {
            let positions = group.map(positions).unwrap_or_default();
            effect_state
                .texture_to_output
                .set_positions(queue, positions);
            effect_state.sent_group = group.cloned();
        }

        effect_state
            .renderer
            .set_buffers(queue, effect_state, palette, &self.animation.config());

        effect_state.texture_to_output.clear_output(queue);
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
