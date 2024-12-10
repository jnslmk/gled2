use crate::{
    app::Timing,
    effect::EffectState,
    group::{GroupIndices, Groups},
    input::InputEvent,
    storage::{Animation, Asset, AssetId, Palette, RangePercentage, Scene, StaticOrCurve},
    transition::Transition,
};
use egui::TextureId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{CommandEncoder, Queue};

#[derive(Serialize, Deserialize, Debug)]
pub struct SceneInstance {
    pub active: bool,
    pub opacity: StaticOrCurve<RangePercentage>,
    pub input_dimmer: f32,
    pub beat_progression_offset: StaticOrCurve<RangePercentage>,
    pub activation_input: Option<InputEvent>,
    pub flash_input: Option<InputEvent>,
    pub dimmer_input: Option<InputEvent>,
    pub scene: AssetId<Scene>,
    pub groups: Option<Groups>,

    #[serde(skip)]
    effect_states: Vec<EffectState>,

    #[serde(skip)]
    transition: Option<Transition>,

    #[serde(skip)]
    pub flash: bool,
}

impl Clone for SceneInstance {
    fn clone(&self) -> Self {
        Self {
            active: self.active,
            opacity: self.opacity,
            input_dimmer: self.input_dimmer,
            beat_progression_offset: self.beat_progression_offset,
            activation_input: self.activation_input,
            flash_input: self.flash_input,
            dimmer_input: self.dimmer_input,
            scene: self.scene,
            groups: self.groups.clone(),
            effect_states: Default::default(),
            transition: Default::default(),
            flash: Default::default(),
        }
    }
}

impl From<AssetId<Scene>> for SceneInstance {
    fn from(scene: AssetId<Scene>) -> Self {
        Self {
            scene,
            active: Default::default(),
            opacity: StaticOrCurve::new_static(1.0),
            input_dimmer: 1.0,
            beat_progression_offset: StaticOrCurve::new_static(0.0),
            activation_input: Default::default(),
            flash_input: Default::default(),
            dimmer_input: Default::default(),
            effect_states: Default::default(),
            transition: Default::default(),
            flash: Default::default(),
            groups: Default::default(),
        }
    }
}

impl SceneInstance {
    pub fn init_states(&mut self) {
        if let Some(scene) = Asset::get(self.scene) {
            scene.data.init_states(&mut self.effect_states);
        }
    }

    pub fn reload_shader_code(&mut self, animation: AssetId<Animation>) {
        if let Some(scene) = Asset::get(self.scene) {
            scene
                .data
                .reload_shader_code(&mut self.effect_states, animation);
        }
    }

    pub fn prepare(
        &mut self,
        queue: &Queue,
        always_render: bool,
        palette: Option<Arc<Asset<Palette>>>,
        deck_groups: &Groups,
        timing: &Timing,
        main_dimmer: f32,
    ) {
        if let Some(event) = self.flash_input.as_ref() {
            self.flash = event.is_live();
        }
        if let Some(event) = self.dimmer_input.as_ref() {
            self.input_dimmer = event.dimmer();
        }

        let mut beat_progression = timing.beat_progression();
        beat_progression += self.beat_progression_offset.value(beat_progression);
        for effect_state in self.effect_states.iter_mut() {
            effect_state.beat_progression = beat_progression;
            effect_state.beats_per_minute = timing.beats_per_minute;
            effect_state.framerate = timing.framerate().unwrap_or_default();
        }

        let mut opacity_factor = 1.0;
        if let Some(transition) = self.transition.as_ref() {
            match transition.opacity_factor() {
                Some(factor) => opacity_factor = factor,
                None => {
                    if transition.goal().turning_off() {
                        self.active = false;
                    }
                    self.transition.take();
                }
            }
        }

        if always_render || self.active || self.flash {
            let groups = self.groups.as_ref().unwrap_or(deck_groups);
            let main_opacity = main_dimmer
                * opacity_factor
                * self.opacity.value(beat_progression)
                * self.input_dimmer;
            if let Some(scene) = Asset::get(self.scene) {
                scene.data.prepare(
                    &mut self.effect_states,
                    queue,
                    palette.clone(),
                    groups,
                    main_opacity,
                );
            }
        }
    }

    pub fn render(&mut self, encoder: &mut CommandEncoder, blackout: bool, always_render: bool) {
        if !self.active && !self.flash && !always_render {
            return;
        }

        let send_output = !blackout && (self.active || self.flash);
        if let Some(scene) = Asset::get(self.scene) {
            scene
                .data
                .render(&mut self.effect_states, encoder, send_output);
        }
    }

    pub fn set_output_mix_buffers(&mut self) {
        if let Some(scene) = Asset::get(self.scene) {
            scene.data.set_output_mix_buffers(&mut self.effect_states);
        }
    }

    pub fn set_transition(&mut self, transition: Transition) {
        self.active = true;
        self.transition = Some(transition);
    }

    pub fn has_transition(&self) -> bool {
        self.transition.is_some()
    }

    pub fn transition_factor(&self) -> f32 {
        self.transition
            .as_ref()
            .and_then(|transition| transition.opacity_factor())
            .unwrap_or(1.0)
    }

    pub fn send_positions(&mut self) {
        for state in self.effect_states.iter_mut() {
            state.send_positions();
        }
    }

    pub fn texture_ids(&self) -> Vec<TextureId> {
        self.effect_states
            .iter()
            .map(|state| state.texture_id())
            .collect()
    }

    pub fn group_indices(&self) -> GroupIndices {
        Asset::get(self.scene)
            .map(|scene| scene.data.group_indices())
            .unwrap_or_default()
    }

    pub fn remove_nonexistant_groups(&mut self) {
        if let Some(groups) = self.groups.as_mut() {
            groups.remove_nonexistant_groups();
        }
    }
}
