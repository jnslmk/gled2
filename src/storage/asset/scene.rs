use super::{Asset, AssetTrait, Palette};
use crate::{
    effect::{Effect, EffectState},
    group::Groups,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{Buffer, CommandEncoder, Queue};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Scene {
    effects: Vec<Effect>,
}

impl Scene {
    pub fn set_buffers(&self, effect_states: &mut [EffectState], output: &Buffer) {
        for (effect, effect_state) in self.effects.iter().zip(effect_states.iter_mut()) {
            effect.set_buffers(effect_state, output);
        }
    }

    pub fn effect(&mut self, index: usize) -> Option<&mut Effect> {
        self.effects.get_mut(index)
    }

    pub fn effects(&mut self) -> &mut [Effect] {
        &mut self.effects
    }

    pub fn add_effect(&mut self, effect_states: &mut Vec<EffectState>, effect: Effect) -> usize {
        effect_states.push(EffectState::new(&effect));
        self.effects.push(effect);

        self.effects.len() - 1
    }

    pub fn remove_effect(
        &mut self,
        effect_states: &mut Vec<EffectState>,
        index: usize,
    ) -> Option<Effect> {
        let mut effect = None;

        if self.effects.len() > index {
            effect = Some(self.effects.remove(index));
            effect_states.remove(index);
        }

        effect
    }

    pub fn init_states(&self, effect_states: &mut Vec<EffectState>) {
        if effect_states.len() != self.effects.len() {
            if effect_states.len() > self.effects.len() {
                effect_states.truncate(self.effects.len());
            } else {
                effect_states.extend(
                    self.effects
                        .iter()
                        .skip(effect_states.len())
                        .map(EffectState::new),
                );
            }
        }

        for (effect, state) in self.effects.iter().zip(effect_states.iter_mut()) {
            state.update(effect);
        }
    }

    pub fn prepare(
        &self,
        effect_states: &mut [EffectState],
        queue: &Queue,
        palette: Option<Arc<Asset<Palette>>>,
        groups: &Groups,
        main_opacity: f32,
    ) {
        debug_assert_eq!(effect_states.len(), self.effects.len());

        for (effect, state) in self.effects.iter().zip(effect_states.iter_mut()) {
            effect.prepare(state, queue, palette.clone(), groups, main_opacity);
        }
    }

    pub fn render(
        &self,
        effect_states: &mut [EffectState],
        encoder: &mut CommandEncoder,
        send_output: bool,
    ) {
        debug_assert_eq!(effect_states.len(), self.effects.len());

        for (effect, state) in self.effects.iter().zip(effect_states.iter_mut()) {
            effect.render(state, encoder, send_output);
        }
    }
}

impl AssetTrait for Scene {
    const DIR_NAME: &'static str = "scenes";
    const NAME: &'static str = "Scene";
}
