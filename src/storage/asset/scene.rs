use super::{Animation, Asset, AssetTrait, Palette};
use crate::{
    effect::{Effect, EffectState},
    group::Groups,
    storage::AssetId,
};
use egui::{epaint::CircleShape, Color32, Label, Rect, Shape, Vec2};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{CommandEncoder, Queue};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Scene {
    effects: Vec<Effect>,
}

impl Scene {
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

    pub fn groups_selection(&self) -> GroupsSelection {
        let mut primary = false;
        let mut secondary = false;

        for state in self.effects.iter() {
            if state.use_secondary_group {
                secondary = true;
            } else {
                primary = true;
            }
        }

        match (primary, secondary) {
            (true, true) => GroupsSelection::Both,
            (true, false) => GroupsSelection::Primary,
            (false, true) => GroupsSelection::Secondary,
            (false, false) => GroupsSelection::None,
        }
    }
}

impl AssetTrait for Scene {
    const DIR_NAME: &'static str = "scenes";
    const NAME: &'static str = "Scene";
    const SHOW_NAME_IF_SELECTED: bool = true;

    fn show(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let groups_selection = self.groups_selection();
        ui.painter().add(Shape::Circle(CircleShape::filled(
            rect.right_top() + Vec2::new(-10.0, 8.0),
            5.5,
            match groups_selection {
                GroupsSelection::None => Color32::RED,
                GroupsSelection::Secondary | GroupsSelection::Both => Color32::GOLD,
                GroupsSelection::Primary => Color32::GREEN,
            },
        )));
        ui.put(
            Rect::from_min_size(rect.right_top() + Vec2::new(-15.0, 1.0), Vec2::splat(10.0)),
            Label::new(
                egui::RichText::new(match groups_selection {
                    GroupsSelection::None => "N",
                    GroupsSelection::Secondary | GroupsSelection::Both => "S",
                    GroupsSelection::Primary => "P",
                })
                .color(Color32::BLACK),
            )
            .selectable(false),
        );
        if let GroupsSelection::Both = groups_selection {
            ui.painter().add(Shape::Circle(CircleShape::filled(
                rect.right_top() + Vec2::new(-21.0, 8.0),
                5.5,
                Color32::GREEN,
            )));
            ui.put(
                Rect::from_min_size(rect.right_top() + Vec2::new(-26.0, 1.0), Vec2::splat(10.0)),
                Label::new(egui::RichText::new("P").color(Color32::BLACK)).selectable(false),
            );
        }
    }
}

pub enum GroupsSelection {
    None,
    Primary,
    Secondary,
    Both,
}
