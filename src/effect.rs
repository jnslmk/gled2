mod state;

use crate::{
    animation::{Animation, AnimationConfig},
    app::positions,
    group::Groups,
    storage::{Asset, Palette},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{Buffer, CommandEncoder, Queue};

pub use state::EffectState;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Effect {
    pub color_shift: f32,
    pub opacity: f32,
    pub beat_progression_offset: f32,
    /// Whether it should use the primary or the secondary group
    pub use_secondary_group: bool,
    pub animation: Animation,
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

impl Clone for Effect {
    fn clone(&self) -> Self {
        Self {
            color_shift: self.color_shift,
            opacity: self.opacity,
            beat_progression_offset: self.beat_progression_offset,
            use_secondary_group: self.use_secondary_group,
            animation: self.animation.clone(),
        }
    }
}

impl Effect {
    pub fn new(animation: Animation) -> Self {
        Self {
            animation,
            opacity: 1.0,
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
        effect_state.opacity = self.opacity * main_opacity;
        effect_state.beat_progression += self.beat_progression_offset;
        effect_state.color_shift = self.color_shift;

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
