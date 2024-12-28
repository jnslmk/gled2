pub mod deck;
pub mod render_deactivated_scenes;
pub mod scene_instance_path;

use deck::{Deck, DeckPath};
use render_deactivated_scenes::RenderDeactivatedScenes;
use scene_instance_path::SceneInstancePath;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, iter::once, time::Duration};
use wgpu::CommandEncoderDescriptor;

use crate::{
    app::{svg::Svg, timing::Timing},
    input::{
        artnet::ArtnetConfig,
        event::{GamepadEvent, InputEvent},
    },
    pipeline::{
        extract_output::ExtractOutput, output_clear::OutputClear, preview::Preview,
        preview_indices::PreviewIndices, renderer_callback::RendererCallback,
    },
    storage::asset_id::AssetId,
    wgpu_render_state,
};

use super::{
    animation::Animation, output_device::routing::OutputRoutings, scene::instance::SceneInstance,
    AssetTrait,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Project {
    a: Deck,
    b: Deck,
    c: Deck,
    /// 0.0 = A, 0.5 = A + B, 1.0 = B
    pub cross_fader: f32,
    pub svg: Option<Svg>,
    pub output_routings: OutputRoutings,
    pub artnet_config: ArtnetConfig,
    pub tap_input_events: BTreeSet<InputEvent>,
    pub blackout_input_events: BTreeSet<InputEvent>,
    pub main_dimmer: f32,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            a: Default::default(),
            b: Default::default(),
            c: Default::default(),
            cross_fader: Default::default(),
            svg: Default::default(),
            output_routings: Default::default(),
            artnet_config: Default::default(),
            tap_input_events: std::iter::once(InputEvent::Key(egui::Key::T))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Mode(0))))
                .collect(),
            blackout_input_events: std::iter::once(InputEvent::Key(egui::Key::B))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Start(0))))
                .collect(),
            main_dimmer: 1.0,
        }
    }
}

impl Project {
    #[inline(always)]
    pub fn decks(&mut self) -> impl Iterator<Item = (SceneInstancePath, &mut Deck)> {
        once((SceneInstancePath::DECK_A, &mut self.a))
            .chain(once((SceneInstancePath::DECK_B, &mut self.b)))
            .chain(once((SceneInstancePath::DECK_C, &mut self.c)))
    }

    #[inline(always)]
    pub fn all_scene_instances(
        &mut self,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        self.decks()
            .flat_map(|(path, deck)| deck.scene_instances(path))
    }

    pub fn reload_shader_code(&mut self, animation: AssetId<Animation>) {
        self.all_scene_instances()
            .for_each(|(_path, scene_instance)| {
                scene_instance.reload_shader_code(animation);
            });
    }

    pub fn send_positions(&mut self) {
        self.all_scene_instances()
            .for_each(|(_path, scene_instance)| {
                scene_instance.send_positions();
            });
    }

    pub fn init_gpu(&mut self) {
        self.all_scene_instances()
            .for_each(|(_path, scene_instance)| {
                scene_instance.init_states();
            });
        self.set_buffers();
    }

    pub fn set_buffers(&mut self) {
        self.all_scene_instances()
            .for_each(|(_path, scene_instance)| {
                scene_instance.set_output_mix_buffers();
            });
        Preview::set_buffers();
    }

    #[inline(always)]
    pub fn deck(&mut self, path: SceneInstancePath) -> &mut Deck {
        match path.deck_path {
            DeckPath::A => &mut self.a,
            DeckPath::B => &mut self.b,
            DeckPath::C => &mut self.c,
        }
    }

    #[inline(always)]
    pub fn scene_instance(&mut self, path: SceneInstancePath) -> Option<&mut SceneInstance> {
        self.deck(path).scene_instance(path)
    }

    #[inline(always)]
    pub fn scene_instances(
        &mut self,
        path: SceneInstancePath,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        self.deck(path).scene_instances(path)
    }

    /// Remove scene instance at path and update path to the next scene instance
    pub fn remove_scene_instance(&mut self, path: &mut SceneInstancePath) {
        self.deck(*path)
            .scenes_instances
            .remove(path.scene_instance);

        while path.scene_instance > 0 {
            if self.scene_instance(*path).is_some() {
                break;
            }
            path.scene_instance -= 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        timing: &Timing,
        blackout: bool,
        render_deactivated_scenes: RenderDeactivatedScenes,
        fade_duration: Duration,
    ) {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;
        let queue = &wgpu_render_state.queue;

        self.a.prepare(
            SceneInstancePath::DECK_A,
            queue,
            timing,
            render_deactivated_scenes,
            fade_duration,
            self.main_dimmer * (1.0 - self.cross_fader),
        );
        self.b.prepare(
            SceneInstancePath::DECK_B,
            queue,
            timing,
            render_deactivated_scenes,
            fade_duration,
            self.main_dimmer * self.cross_fader,
        );
        self.c.prepare(
            SceneInstancePath::DECK_C,
            queue,
            timing,
            render_deactivated_scenes,
            fade_duration,
            self.main_dimmer,
        );

        PreviewIndices::get().prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        OutputClear::get().run(&mut encoder);

        self.decks().for_each(|(path, deck)| {
            deck.render(path, &mut encoder, blackout, render_deactivated_scenes)
        });

        ExtractOutput::get().run(&mut encoder);
        PreviewIndices::get().run(&mut encoder);
        Preview::run(&mut encoder);

        RendererCallback::add(encoder.finish());
    }

    pub fn tap_input_is_new(&self) -> bool {
        self.tap_input_events.iter().any(|event| event.is_new())
    }

    pub fn blackout_input_is_new(&self) -> bool {
        self.blackout_input_events
            .iter()
            .any(|event| event.is_new())
    }

    pub fn remove_nonexistant_groups(&mut self) {
        self.decks().for_each(|(_, deck)| {
            deck.remove_nonexistant_groups();
        });
    }
}

impl AssetTrait for Project {
    const DIR_NAME: &'static str = "projects";
    const NAME: &'static str = "Project";
    const SHOW_NAME_IF_SELECTED: bool = true;
}
