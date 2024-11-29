use super::{Animation, Asset, AssetTrait, Palette};
use crate::{
    effect::{Effect, EffectState},
    group::{GroupIndices, Groups},
    storage::AssetId,
    ui::pills::show_pills,
};
use egui::{Color32, Vec2};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{CommandEncoder, Queue};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Scene {
    effects: Vec<Effect>,
}

impl Scene {
    pub fn group_indices(&self) -> GroupIndices {
        self.effects
            .iter()
            .map(|effect| effect.group_index)
            .collect()
    }

    pub fn set_output_mix_buffers(&self, effect_states: &mut [EffectState]) {
        for (effect, effect_state) in self.effects.iter().zip(effect_states.iter_mut()) {
            effect.set_output_mix_buffers(effect_state);
        }
    }

    pub fn effect(&mut self, index: usize) -> Option<&mut Effect> {
        self.effects.get_mut(index)
    }

    pub fn effects(&self) -> &[Effect] {
        &self.effects
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
                let previous_len = effect_states.len();
                effect_states.extend(self.effects.iter().skip(previous_len).map(EffectState::new));
                for (effect, state) in self
                    .effects
                    .iter()
                    .zip(effect_states.iter_mut())
                    .skip(previous_len)
                {
                    state.update(effect);
                }
            }
        }
    }

    pub fn reload_shader_code(
        &self,
        effect_states: &mut [EffectState],
        animation: AssetId<Animation>,
    ) {
        for (effect, state) in self.effects.iter().zip(effect_states.iter_mut()) {
            if effect.animation == Some(animation) {
                state.update(effect);
            }
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
    const SHOW_NAME_IF_SELECTED: bool = true;

    fn show(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        show_pills(
            ui,
            rect.right_top() + Vec2::new(0.0, 1.0),
            self.group_indices()
                .into_iter()
                .map(|index| (index.to_string(), Color32::GOLD))
                .collect(),
        );
    }
}
