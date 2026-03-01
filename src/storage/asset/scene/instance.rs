use super::Scene;
use crate::{
    app::timing::Timing,
    audio::sound_trigger_data::SoundTriggerData,
    input::event::InputEvent,
    pipeline::{
        group::{GroupIndices, Groups},
        transition::Transition,
    },
    storage::{
        Asset, AssetId,
        asset::{animation::Animation, scene::color::SceneInstanceColor},
        collections::Collections,
        curve::multiplied_curve::{MultipliedCurve, RangePercentage},
        palette::Palette,
    },
};
use egui::TextureId;
use egui_dnd::DragDropItem;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use wgpu::{CommandEncoder, Queue};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SceneInstance {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "unnamed_scene")]
    pub name: String,

    #[serde(default)]
    pub color: SceneInstanceColor,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub opacity: MultipliedCurve<RangePercentage>,
    #[serde(default)]
    pub input_dimmer: f32,
    #[serde(default)]
    pub ignore_main_dimmer: bool,
    #[serde(default)]
    pub beat_progression_offset: MultipliedCurve<RangePercentage>,
    #[serde(default)]
    pub activation_input: Option<InputEvent>,
    #[serde(default)]
    pub flash_input: Option<InputEvent>,
    #[serde(default)]
    pub set_offset_on_flash: bool,
    #[serde(default)]
    pub dimmer_input: Option<InputEvent>,
    pub scene_id: AssetId<Scene>,
    pub scene: Scene,
    #[serde(default)]
    pub groups_overwrite: Option<Groups>,
    #[serde(default)]
    pub palette_overwrite: Option<Option<AssetId<Palette>>>,

    #[serde(skip)]
    transition: Option<Transition>,

    #[serde(skip)]
    pub flash: bool,
}

fn unnamed_scene() -> String {
    "Unnamed Scene".to_string()
}

impl Clone for SceneInstance {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            id: self.id,
            color: self.color,
            active: self.active,
            opacity: self.opacity.clone(),
            input_dimmer: self.input_dimmer,
            ignore_main_dimmer: self.ignore_main_dimmer,
            beat_progression_offset: self.beat_progression_offset.clone(),
            activation_input: self.activation_input,
            flash_input: self.flash_input,
            set_offset_on_flash: self.set_offset_on_flash,
            dimmer_input: self.dimmer_input,
            scene_id: self.scene_id,
            scene: self.scene.clone(),
            groups_overwrite: self.groups_overwrite.clone(),
            palette_overwrite: self.palette_overwrite,
            transition: Default::default(),
            flash: Default::default(),
        }
    }
}

impl DragDropItem for &mut SceneInstance {
    fn id(&self) -> egui::Id {
        egui::Id::new(self.id)
    }
}

impl SceneInstance {
    pub fn from_scene_id(scene_id: AssetId<Scene>, collections: &Collections) -> Self {
        let asset = Asset::get(scene_id, collections).unwrap_or_default();
        let mut scene = asset.data.clone();
        scene.reload_shader_code(None, collections);

        Self {
            scene_id,
            scene,
            name: asset
                .path
                .last()
                .map(|s| s.to_string())
                .unwrap_or_else(unnamed_scene),
            color: Default::default(),
            id: Uuid::new_v4(),
            active: Default::default(),
            opacity: MultipliedCurve::new_multiplier(1.0),
            input_dimmer: 1.0,
            ignore_main_dimmer: false,
            beat_progression_offset: MultipliedCurve::new_multiplier(0.0),
            activation_input: Default::default(),
            flash_input: Default::default(),
            set_offset_on_flash: Default::default(),
            dimmer_input: Default::default(),
            transition: Default::default(),
            flash: Default::default(),
            groups_overwrite: Default::default(),
            palette_overwrite: Default::default(),
        }
    }

    /// Reload shader code for all effects using the given animation, should be called after an animation is edited
    /// If the given animation is None, reloads all effects
    pub fn reload_shader_code(
        &mut self,
        animation: Option<AssetId<Animation>>,
        collections: &Collections,
    ) {
        self.scene.reload_shader_code(animation, collections);
    }

    pub fn prepare(
        &mut self,
        queue: &Queue,
        always_render: bool,
        palette: Option<Arc<Asset<Palette>>>,
        deck_groups: &Groups,
        timing: &Timing,
        main_dimmer: f32,
        collections: &Collections,
        sound_trigger_data: &SoundTriggerData,
    ) {
        if let Some(event) = self.flash_input.as_ref() {
            if !self.flash && event.is_live() && self.set_offset_on_flash {
                self.beat_progression_offset =
                    MultipliedCurve::new_multiplier(4.0 - timing.beat_progression() % 4.0);
            }
            self.flash = event.is_live();
        }
        if let Some(event) = self.dimmer_input.as_ref() {
            self.input_dimmer = event.dimmer();
        }

        let mut beat_progression = timing.beat_progression();
        beat_progression +=
            self.beat_progression_offset
                .value(beat_progression, collections, sound_trigger_data);
        for effect in self.scene.effects.iter_mut() {
            effect.state.beat_progression = beat_progression;
            effect.state.beats_per_minute = timing.beats_per_minute();
            effect.state.framerate = timing.framerate().unwrap_or_default();
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
            let groups = self.groups_overwrite.as_ref().unwrap_or(deck_groups);
            let palette = match self.palette_overwrite.as_ref() {
                Some(id) => id.and_then(|id| Asset::get(id, collections)),
                None => palette,
            };
            let main_opacity = if self.ignore_main_dimmer {
                1.0
            } else {
                main_dimmer
            } * opacity_factor
                * self
                    .opacity
                    .value(beat_progression, collections, sound_trigger_data)
                * self.input_dimmer;
            self.scene.prepare(
                queue,
                palette,
                groups,
                main_opacity,
                collections,
                sound_trigger_data,
            );
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn render(&mut self, encoder: &mut CommandEncoder, blackout: bool, always_render: bool) {
        if !self.active && !self.flash && !always_render {
            return;
        }

        let send_output = !blackout && (self.active || self.flash);
        self.scene.render(encoder, send_output);
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
        for effect in self.scene.effects.iter_mut() {
            effect.state.send_positions();
        }
    }

    pub fn texture_ids(&self) -> Vec<TextureId> {
        self.scene
            .effects
            .iter()
            .flat_map(|effect| effect.state.texture_id())
            .collect()
    }

    pub fn group_indices(&self) -> GroupIndices {
        self.scene.group_indices()
    }

    pub fn remove_nonexistant_groups(&mut self) {
        if let Some(groups) = self.groups_overwrite.as_mut() {
            groups.remove_nonexistant_groups();
        }
    }

    pub fn set_new_id(&mut self) {
        self.id = Uuid::new_v4();
    }
}
