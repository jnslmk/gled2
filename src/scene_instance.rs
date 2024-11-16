use crate::{
    app::{PersistantState, Timing},
    effect::EffectState,
    group::Groups,
    input::InputEvent,
    storage::{Asset, AssetId, GroupsSelection, Palette, RangePercentage, Scene, StaticOrCurve},
    transition::Transition,
};
use egui::TextureId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{Buffer, CommandEncoder, Queue};

#[derive(Serialize, Deserialize, Debug)]
pub struct SceneInstance {
    ///TODO: Remove as this should eventually come frome the SceneGroup
    pub groups: Groups,
    pub active: bool,
    pub opacity: StaticOrCurve<RangePercentage>,
    pub input_dimmer: f32,
    pub beat_progression_offset: StaticOrCurve<RangePercentage>,
    pub selection_input: Option<InputEvent>,
    pub flash_input: Option<InputEvent>,
    pub dimmer_input: Option<InputEvent>,
    pub scene: AssetId<Scene>,

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
            groups: self.groups.clone(),
            active: self.active,
            opacity: self.opacity,
            input_dimmer: self.input_dimmer,
            beat_progression_offset: self.beat_progression_offset,
            selection_input: self.selection_input,
            flash_input: self.flash_input,
            dimmer_input: self.dimmer_input,
            scene: self.scene,
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
            groups: Default::default(),
            active: Default::default(),
            opacity: StaticOrCurve::new_static(1.0),
            input_dimmer: 1.0,
            beat_progression_offset: StaticOrCurve::new_static(0.0),
            selection_input: Default::default(),
            flash_input: Default::default(),
            dimmer_input: Default::default(),
            effect_states: Default::default(),
            transition: Default::default(),
            flash: Default::default(),
        }
    }
}

impl SceneInstance {
    pub fn init_states(&mut self) {
        if let Some(scene) = Asset::get(self.scene) {
            scene.data.init_states(&mut self.effect_states);
        }
    }

    pub fn prepare(
        &mut self,
        queue: &Queue,
        always_render: bool,
        palette: Option<Arc<Asset<Palette>>>,
        timing: &Timing,
    ) {
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
            let main_opacity = PersistantState::main_dimmer()
                * opacity_factor
                * self.opacity.value(beat_progression)
                * self.input_dimmer;
            if let Some(scene) = Asset::get(self.scene) {
                scene.data.prepare(
                    &mut self.effect_states,
                    queue,
                    palette.clone(),
                    &self.groups,
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

    pub fn set_buffers(&mut self, output: &Buffer) {
        if let Some(scene) = Asset::get(self.scene) {
            scene.data.set_buffers(&mut self.effect_states, output);
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

    pub fn set_flash(&mut self, flash: bool) {
        self.flash = flash;
    }

    pub fn send_positions(&mut self) {
        for state in self.effect_states.iter_mut() {
            state.send_positions();
        }
    }

    pub fn set_input_dimmer(&mut self, input_dimmer: f32) {
        self.input_dimmer = input_dimmer;
    }

    pub fn texture_ids(&self) -> Vec<TextureId> {
        self.effect_states
            .iter()
            .map(|state| state.texture_id())
            .collect()
    }

    pub fn groups_selection(&self) -> GroupsSelection {
        if let Some(scene) = Asset::get(self.scene) {
            scene.data.groups_selection()
        } else {
            GroupsSelection::None
        }
    }
}
