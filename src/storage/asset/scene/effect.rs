use super::effect_state::EffectState;
use crate::{
    app::svg::Svg,
    audio::sound_trigger_data::SoundTriggerData,
    pipeline::group::Groups,
    storage::{
        Animation, Asset, AssetId, Palette,
        animation::config::AnimationConfig,
        collections::Collections,
        curve::multiplied_curve::{MultipliedCurve, RangeDegrees, RangePercentage},
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{CommandEncoder, Queue};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Effect {
    pub color_shift: MultipliedCurve<RangeDegrees>,
    pub opacity: MultipliedCurve<RangePercentage>,
    pub beat_progression: MultipliedCurve<RangePercentage>,
    pub beat_progression_offset: MultipliedCurve<RangePercentage>,
    /// In 2^n of bpm
    pub speed_exponent: i32,
    pub group_index: usize,
    pub animation: Option<AssetId<Animation>>,
    pub animation_config: AnimationConfig,

    /// Overwrite the animation with this one, must be only used in the code editor/preview
    #[serde(skip)]
    pub animation_overwrite: Option<Arc<Asset<Animation>>>,

    #[serde(skip)]
    pub state: EffectState,
}

impl Eq for Effect {}

impl Clone for Effect {
    fn clone(&self) -> Self {
        Self {
            color_shift: self.color_shift.clone(),
            opacity: self.opacity.clone(),
            beat_progression: self.beat_progression.clone(),
            beat_progression_offset: self.beat_progression_offset.clone(),
            speed_exponent: self.speed_exponent,
            group_index: self.group_index,
            animation: self.animation,
            animation_config: self.animation_config.clone(),
            animation_overwrite: self.animation_overwrite.clone(),
            state: EffectState::default(),
        }
    }
}

impl Default for Effect {
    fn default() -> Self {
        Self {
            color_shift: MultipliedCurve::new_multiplier(0.0),
            opacity: MultipliedCurve::new_multiplier(1.0),
            beat_progression: MultipliedCurve::new_curve([
                0x2E, 0x07, 0x5C, 0xCB, 0x90, 0xC4, 0x47, 0x4B, 0x8D, 0x16, 0xC6, 0x70, 0x40, 0x54,
                0x99, 0xA6,
            ]),
            beat_progression_offset: MultipliedCurve::new_multiplier(0.0),
            speed_exponent: Default::default(),
            group_index: Default::default(),
            animation: Default::default(),
            animation_config: Default::default(),
            animation_overwrite: Default::default(),
            state: Default::default(),
        }
    }
}

impl Effect {
    fn default_shader_code_complete() -> String {
        format!(
            "{}\n\n{}",
            include_str!("../../../shaders/common.wgsl"),
            include_str!("../../../shaders/red.wgsl")
        )
    }

    pub fn shader_code_complete(&self, collections: &Collections) -> String {
        self.animation_overwrite
            .clone()
            .or_else(|| self.animation.and_then(|id| Asset::get(id, collections)))
            .map(|animation| animation.data.shader_code_complete())
            .unwrap_or_else(Self::default_shader_code_complete)
    }

    pub fn prepare(
        &mut self,
        queue: &Queue,
        palette: Option<Arc<Asset<Palette>>>,
        groups: &Groups,
        main_opacity: f32,
        collections: &Collections,
        sound_trigger_data: &SoundTriggerData,
    ) {
        let beat_progression = self.state.beat_progression
            + self.beat_progression_offset.value(
                self.state.beat_progression,
                collections,
                sound_trigger_data,
            );
        self.state.beat_progression =
            self.beat_progression
                .value(beat_progression, collections, sound_trigger_data);
        self.state.opacity = self
            .opacity
            .value(beat_progression, collections, sound_trigger_data)
            * main_opacity;
        self.state.color_shift =
            self.color_shift
                .value(beat_progression, collections, sound_trigger_data);
        self.state.speed_exponent = self.speed_exponent;
        self.state.animation_config = self.animation_config.clone();

        let group = groups.get(self.group_index);
        if self.state.sent_group.as_ref() != group {
            let positions = group.map(Svg::positions);
            if let Some(texture_to_output) = self.state.texture_to_output() {
                texture_to_output.set_positions(queue, positions.unwrap_or_default());
            }
            self.state.sent_group = group.cloned();
        }

        if let Some(renderer) = self.state.renderer() {
            renderer.set_buffers(
                queue,
                &self.state,
                beat_progression,
                palette,
                collections,
                sound_trigger_data,
            );
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn render(&self, encoder: &mut CommandEncoder, send_output: bool) {
        self.state.render(encoder, send_output);
    }
}
