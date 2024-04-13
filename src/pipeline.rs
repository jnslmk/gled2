use crate::{
    animation::{
        Color, ColorPalette, CommonConfig, Direction, Gradient, GradientType, State, Stripes,
    },
    constants::{GPU_NOT_INIT, OUTPUT_BUFFER_SIZE},
    effect::Effect,
    extract_output::ExtractOutput,
    output_clear::OutputClear,
    output_sender::{GpuReadyReceiver, OutputSender},
    preview::Preview,
    preview_indices::PreviewIndices,
    svg::Universes,
    transition::{Transition, TransitionGoal},
    wgpu_render_state,
};
use egui::TextureId;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Pipeline {
    effects: Vec<Effect>,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_effects: usize,

    #[serde(skip)]
    auto_mode_last_change: Option<Instant>,
    #[serde(skip)]
    start: Option<Instant>,
    #[serde(skip)]
    extract: Option<ExtractOutput>,
    #[serde(skip)]
    preview_indices: Option<PreviewIndices>,
    #[serde(skip)]
    preview: Option<Preview>,
    #[serde(skip)]
    output: Option<Buffer>,
    #[serde(skip)]
    output_clear: Option<OutputClear>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            effects: Default::default(),
            auto_mode_last_change: Default::default(),
            auto_mode_active: false,
            auto_mode_seconds: 45,
            auto_mode_max_effects: 3,
            start: Default::default(),
            extract: Default::default(),
            preview_indices: Default::default(),
            preview: Default::default(),
            output: Default::default(),
            output_clear: Default::default(),
        }
    }
}

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        Self {
            effects: self.effects.clone(),
            ..Default::default()
        }
    }
}

impl Pipeline {
    pub fn demo() -> Self {
        let mut pipeline = Self::default();

        for i in 0..10 {
            let palette = ColorPalette {
                colors: vec![Color::new(1., 0., 0.), Color::new(0., 0., 0.)],
            };
            let gradient = Gradient {
                gradient: GradientType::Radial,
                center: (0.25, 0.5),
                ..Default::default()
            };
            let mut effect = Effect::new(gradient.into(), palette, "allFull".to_owned());
            effect.active = i == 0;
            pipeline.add_effect(effect);

            let palette = ColorPalette {
                colors: vec![Color::new(0., 0., 1.), Color::new(0., 0., 0.)],
            };
            let gradient = Gradient {
                gradient: GradientType::LinearHorizontal,
                common: CommonConfig {
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut effect = Effect::new(gradient.into(), palette, "innerFull".to_owned());
            effect.active = i == 0;
            pipeline.add_effect(effect);

            let palette = ColorPalette {
                colors: vec![Color::new(1., 1., 0.)],
            };
            let stripes = Stripes {
                count: 2,
                common: CommonConfig {
                    direction: Direction::Backward,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut effect = Effect::new(stripes.into(), palette, "innerEdge".to_owned());
            effect.active = i == 0;
            pipeline.add_effect(effect);
        }

        pipeline
    }

    pub fn set_extract_output(&mut self, extract_output: ExtractOutput) {
        self.extract = Some(extract_output);
    }

    pub fn init_gpu(&mut self) {
        self.output_clear.get_or_insert_with(OutputClear::init);
        self.output.get_or_insert_with(|| {
            wgpu_render_state().device.create_buffer(&BufferDescriptor {
                size: OUTPUT_BUFFER_SIZE,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
                label: Some("Output buffer"),
                mapped_at_creation: false,
            })
        });
        self.preview_indices.get_or_insert_with(PreviewIndices::new);
        self.preview.get_or_insert_with(Preview::new);
        for effect in self.effects.iter_mut() {
            effect.init_gpu();
        }
        self.set_buffers();
    }

    pub fn start(&mut self) -> Instant {
        *self.start.get_or_insert_with(Instant::now)
    }

    pub fn add_effect(&mut self, effect: Effect) -> usize {
        self.effects.push(effect);
        self.init_gpu();

        self.effects.len() - 1
    }

    pub fn remove_effect(&mut self, index: usize) -> Option<Effect> {
        let mut effect = None;

        if self.effects.len() > index {
            effect = Some(self.effects.remove(index));
            self.init_gpu();
        }

        effect
    }

    pub fn set_buffers(&mut self) {
        for effect in self.effects.iter_mut() {
            effect.set_buffers(self.output.as_ref().expect(GPU_NOT_INIT))
        }

        self.preview.as_mut().expect(GPU_NOT_INIT).set_buffers(
            self.preview_indices.as_ref().expect(GPU_NOT_INIT).indices(),
            self.output.as_ref().expect(GPU_NOT_INIT),
        );

        self.output_clear
            .as_mut()
            .expect(GPU_NOT_INIT)
            .set_buffers(self.output.as_ref().expect(GPU_NOT_INIT))
    }

    pub fn set_opacity(&mut self, index: usize, opacity: f32) {
        if let Some(effect) = self.effect(index) {
            effect.opacity = opacity;
        }
    }

    pub fn set_active(&mut self, index: usize, active: bool) {
        if let Some(effect) = self.effect(index) {
            effect.active = active;
        }
    }

    pub fn effects(&mut self) -> Vec<(usize, &mut Effect)> {
        self.effects.iter_mut().enumerate().collect()
    }

    pub fn effect(&mut self, index: usize) -> Option<&mut Effect> {
        self.effects.get_mut(index)
    }

    pub fn svg_or_groups_changed(&mut self, universes: Universes) {
        for effect in self.effects.iter_mut() {
            effect.send_positions();
        }
        self.preview_indices
            .as_mut()
            .expect(GPU_NOT_INIT)
            .send_positions();
        self.extract
            .as_mut()
            .expect(GPU_NOT_INIT)
            .set_universes(universes);
    }

    pub fn preview_texture_id(&self) -> TextureId {
        self.preview.as_ref().expect(GPU_NOT_INIT).texture_id()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        output_sender: &mut OutputSender,
        gpu_ready_receiver: &mut GpuReadyReceiver,
        beat_progression: f32,
        beats_per_minute: f32,
        framerate: f32,
        blackout: bool,
        main_dimmer: f32,
        render_deactivated_effects: RenderDeactivatedEffects,
        fade_duration: Duration,
    ) {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;
        let queue = &wgpu_render_state.queue;

        let state = State {
            beat_progression,
            beats_per_minute,
            framerate,
            ..Default::default()
        };

        if self.auto_mode_active {
            if self
                .auto_mode_last_change
                .get_or_insert_with(Instant::now)
                .elapsed()
                .as_secs()
                > self.auto_mode_seconds
            {
                let auto_mode_max_effects = self.auto_mode_max_effects;
                let mut prev = HashSet::new();
                {
                    let mut indices = self
                        .effects()
                        .into_iter()
                        .filter(|(_index, effect)| effect.active)
                        .map(|(index, _effect)| index)
                        .collect::<Vec<_>>();

                    let mut disable_count =
                        (indices.len() + 1).saturating_sub(auto_mode_max_effects);
                    while disable_count > 0 {
                        if let Some(index) = indices.choose_mut(&mut rand::thread_rng()).copied() {
                            if prev.insert(index) {
                                disable_count -= 1;
                                if let Some(effect) = self.effect(index) {
                                    effect.set_transition(Transition::new(
                                        TransitionGoal::TurnOff,
                                        fade_duration,
                                    ));
                                }
                            }
                        }
                    }
                }

                let mut effects = self
                    .effects()
                    .into_iter()
                    .filter(|(index, _effect)| !prev.contains(index))
                    .collect::<Vec<_>>();
                if let Some((_index, effect)) = effects.choose_mut(&mut rand::thread_rng()) {
                    effect.set_transition(Transition::new(TransitionGoal::TurnOn, fade_duration));
                }

                self.auto_mode_last_change.take();
            }
        } else {
            self.auto_mode_last_change.take();
        }

        for (index, effect) in self.effects() {
            effect.prepare(
                queue,
                state,
                blackout,
                main_dimmer,
                render_deactivated_effects.should_render(index),
            );
        }
        self.preview_indices
            .as_mut()
            .expect(GPU_NOT_INIT)
            .prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        self.output_clear
            .as_ref()
            .expect(GPU_NOT_INIT)
            .run(&mut encoder);

        for (index, effect) in self.effects() {
            effect.render(
                &mut encoder,
                blackout,
                render_deactivated_effects.should_render(index),
            );
        }

        self.extract
            .as_mut()
            .expect(GPU_NOT_INIT)
            .run(&mut encoder, self.output.as_ref().expect(GPU_NOT_INIT));
        self.preview_indices
            .as_mut()
            .expect(GPU_NOT_INIT)
            .run(&mut encoder);
        self.preview.as_mut().expect(GPU_NOT_INIT).run(&mut encoder);

        //wait for gpu to be ready for the next queue submission
        gpu_ready_receiver.recv().ok();
        queue.submit(std::iter::once(encoder.finish()));

        output_sender
            .send(())
            .expect("Output sender closed its channel");
    }
}

pub enum RenderDeactivatedEffects {
    Always,
    Some(usize, usize),
}

impl RenderDeactivatedEffects {
    pub fn should_render(&self, index: usize) -> bool {
        match self {
            RenderDeactivatedEffects::Always => true,
            RenderDeactivatedEffects::Some(selected, hovered) => {
                *selected == index || *hovered == index
            }
        }
    }
}
