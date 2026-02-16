pub mod color;
pub mod effect;
pub mod effect_state;
pub(crate) mod grid;
pub mod instance;

use super::{Asset, AssetTrait, animation::Animation, palette::Palette};
use crate::{
    pipeline::group::{GroupIndices, Groups},
    storage::AssetId,
    ui::pills::show_pills,
};
use effect::Effect;
use egui::{Color32, Vec2};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{CommandEncoder, Queue};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Scene {
    pub effects: Vec<Effect>,
}

impl Scene {
    pub fn group_indices(&self) -> GroupIndices {
        self.effects
            .iter()
            .map(|effect| effect.group_index)
            .collect()
    }

    pub fn effect(&mut self, index: usize) -> Option<&mut Effect> {
        self.effects.get_mut(index)
    }

    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    pub fn add_effect(&mut self, effect: Effect) -> usize {
        self.effects.push(effect);

        self.effects.len() - 1
    }

    pub fn remove_effect(&mut self, index: usize) -> Option<Effect> {
        let mut effect = None;

        if self.effects.len() > index {
            effect = Some(self.effects.remove(index));
        }

        effect
    }

    /// Reload shader code for all effects using the given animation, should be called after an animation is edited
    /// If the given animation is None, reloads all effects
    pub fn reload_shader_code(&mut self, animation: Option<AssetId<Animation>>) {
        for effect in self.effects.iter_mut() {
            if let Some(animation) = animation
                && effect.animation == Some(animation)
            {
                continue;
            }

            effect.state.set_shader_code(&effect.shader_code_complete());
        }
    }

    pub fn prepare(
        &mut self,
        queue: &Queue,
        palette: Option<Arc<Asset<Palette>>>,
        groups: &Groups,
        main_opacity: f32,
    ) {
        for effect in self.effects.iter_mut() {
            effect.prepare(queue, palette.clone(), groups, main_opacity);
        }
    }

    pub fn render(&mut self, encoder: &mut CommandEncoder, send_output: bool) {
        for effect in self.effects.iter_mut() {
            effect.render(encoder, send_output);
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
