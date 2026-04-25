use crate::{
    audio::sound_data::SoundData,
    pipeline::group::Groups,
    storage::{
        asset::scene::{Scene, effect::Effect},
        collections::Collections,
    },
    ui::effect::widget::EffectWidget,
};
use egui::{Button, Color32, Ui, Vec2};

#[derive(Default)]
pub struct SceneEffectEditorState {
    pub selected_effect: usize,
}

impl SceneEffectEditorState {
    pub fn reset(&mut self) {
        self.selected_effect = 0;
    }

    pub fn clamp(&mut self, effect_count: usize) {
        if effect_count == 0 {
            self.selected_effect = 0;
        } else {
            self.selected_effect = self.selected_effect.min(effect_count - 1);
        }
    }
}

pub fn selected_effect_editor_ui(
    ui: &mut Ui,
    scene: &mut Scene,
    state: &mut SceneEffectEditorState,
    allow_animation_change: bool,
    svg: Option<egui::TextureHandle>,
    beat_progression: f32,
    collections: &mut Collections,
    sound_data: &mut SoundData,
) -> bool {
    state.clamp(scene.effects().len());

    let mut changed = false;
    if let Some(effect) = scene.effect(state.selected_effect) {
        changed |= effect.config_ui(
            ui,
            allow_animation_change,
            svg,
            beat_progression,
            collections,
            sound_data,
        );
    }

    ui.separator();

    ui.vertical_centered_justified(|ui| {
        if ui
            .add(Button::new("+ Add Animation").fill(Color32::DARK_GREEN))
            .clicked()
        {
            state.selected_effect = scene.add_effect(Effect::default(), collections);
            changed = true;
        }
    });
    ui.vertical_centered_justified(|ui| {
        if ui
            .add(Button::new("🗐 Duplicate Animation").fill(Color32::DARK_BLUE))
            .clicked()
            && let Some(effect) = scene.effect(state.selected_effect).cloned()
        {
            state.selected_effect = scene.add_effect(effect, collections);
            changed = true;
        }
    });
    ui.vertical_centered_justified(|ui| {
        if ui
            .add(Button::new("🗑 Remove Animation").fill(Color32::DARK_RED))
            .clicked()
        {
            scene.remove_effect(state.selected_effect);
            state.selected_effect = state.selected_effect.saturating_sub(1);
            state.clamp(scene.effects().len());
            changed = true;
        }
    });

    changed
}

pub fn scene_effect_list_ui(
    ui: &mut Ui,
    scene: &Scene,
    state: &mut SceneEffectEditorState,
    show_group: bool,
    svg: Option<egui::TextureHandle>,
    groups: Option<&Groups>,
    groups_show_index: bool,
    beat_progression: Option<f32>,
    collections: &mut Collections,
    sound_data: &mut SoundData,
) {
    state.clamp(scene.effects().len());

    ui.horizontal_wrapped(|ui| {
        for (index, effect) in scene.effects().iter().enumerate() {
            ui.add_sized(
                Vec2::splat(100.0),
                EffectWidget {
                    show_group,
                    selectable: Some((&mut state.selected_effect, index)),
                    effect,
                    svg: svg.clone(),
                    groups,
                    groups_show_index,
                    beat_progression,
                    collections,
                    sound_data,
                },
            );
        }
    });
}
