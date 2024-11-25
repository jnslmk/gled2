mod deck;
mod render_deactivated_scenes;
mod scene_group;
mod scene_instance_path;

use super::{Animation, AssetId, AssetTrait};
use crate::{
    app::{Svg, Timing},
    extract_output::ExtractOutput,
    input::{ArtnetConfig, GamepadEvent, InputEvent},
    output_clear::OUTPUT_CLEAR,
    output_routings::OutputRoutings,
    output_sender::{GpuReadyReceiver, OutputSender},
    preview::PREVIEW,
    preview_indices::PREVIEW_INDICES,
    scene_instance::SceneInstance,
    wgpu_render_state,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, iter::once, time::Duration};
use wgpu::CommandEncoderDescriptor;

pub use deck::Deck;
pub use render_deactivated_scenes::RenderDeactivatedScenes;
pub use scene_group::SceneGroup;
pub use scene_instance_path::SceneInstancePath;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Project {
    pub a: Deck,
    pub b: Deck,
    /// 0.0 = A, 0.5 = A + B, 1.0 = B
    pub cross_fader: f32,
    pub svg: Option<Svg>,
    pub output_routings: OutputRoutings,
    pub artnet_config: ArtnetConfig,
    pub tap_input_events: BTreeSet<InputEvent>,
    pub freeze_input_events: BTreeSet<InputEvent>,
    pub blackout_input_events: BTreeSet<InputEvent>,
    pub main_dimmer: f32,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            a: Deck {
                scene_groups: vec![Default::default()],
                palette: None,
                auto_mode_active: Default::default(),
                auto_mode_seconds: Default::default(),
                auto_mode_max_scenes: Default::default(),
                auto_mode_last_change: Default::default(),
            },
            b: Default::default(),
            cross_fader: Default::default(),
            svg: Default::default(),
            output_routings: Default::default(),
            artnet_config: Default::default(),
            tap_input_events: std::iter::once(InputEvent::Key(egui::Key::T))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Mode(0))))
                .collect(),
            freeze_input_events: std::iter::once(InputEvent::Key(egui::Key::F))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Select(
                    0,
                ))))
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
    }

    #[inline(always)]
    pub fn all_scene_instances(
        &mut self,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        self.decks().flat_map(|(path, deck)| {
            deck.scene_groups(path)
                .flat_map(|(path, scene_groups)| scene_groups.scene_instances(path))
        })
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
        PREVIEW.lock().set_buffers();
    }

    #[inline(always)]
    pub fn deck(&mut self, path: SceneInstancePath) -> &mut Deck {
        if path.deck_a {
            &mut self.a
        } else {
            &mut self.b
        }
    }

    #[inline(always)]
    pub fn scene_group(&mut self, path: SceneInstancePath) -> Option<&mut SceneGroup> {
        self.deck(path).scene_group(path)
    }

    #[inline(always)]
    pub fn scene_instance(&mut self, path: SceneInstancePath) -> Option<&mut SceneInstance> {
        self.scene_group(path)?.scene_instance(path)
    }

    #[inline(always)]
    pub fn scene_instances(
        &mut self,
        path: SceneInstancePath,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        if path.deck_a {
            self.a.scene_instances(path)
        } else {
            self.b.scene_instances(path)
        }
    }

    /// Remove scene instance at path and update path to the next scene instance
    pub fn remove_scene_instance(&mut self, path: &mut SceneInstancePath) {
        if let Some(scene_group) = self.scene_group(*path) {
            scene_group.scenes_instances.remove(path.scene_instance);
        }
        while path.scene_instance > 0 {
            if self.scene_instance(*path).is_some() {
                break;
            }
            path.scene_instance -= 1;
        }
        while path.scene_group > 0 {
            if self.scene_instance(*path).is_some() {
                break;
            }
            path.scene_group -= 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        output_sender: &mut OutputSender,
        gpu_ready_receiver: &mut GpuReadyReceiver,
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

        PREVIEW_INDICES.lock().prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        OUTPUT_CLEAR.lock().run(&mut encoder);

        self.decks().for_each(|(path, deck)| {
            deck.render(path, &mut encoder, blackout, render_deactivated_scenes)
        });

        ExtractOutput::get().run(&mut encoder);
        PREVIEW_INDICES.lock().run(&mut encoder);
        PREVIEW.lock().run(&mut encoder);

        //wait for gpu to be ready for the next queue submission
        gpu_ready_receiver.recv().ok();
        queue.submit(once(encoder.finish()));

        output_sender
            .send(())
            .expect("Output sender closed its channel");
    }

    pub fn tap_input_is_new(&self) -> bool {
        self.tap_input_events.iter().any(|event| event.is_new())
    }

    pub fn freeze_input_is_new(&self) -> bool {
        self.freeze_input_events.iter().any(|event| event.is_new())
    }

    pub fn blackout_input_is_new(&self) -> bool {
        self.blackout_input_events
            .iter()
            .any(|event| event.is_new())
    }
}

impl AssetTrait for Project {
    const DIR_NAME: &'static str = "projects";
    const NAME: &'static str = "Project";
    const SHOW_NAME_IF_SELECTED: bool = true;
}
