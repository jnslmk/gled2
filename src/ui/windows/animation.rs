use crate::{
    app::{PersistantState, Timing},
    effect::{Effect, EffectState},
    group::Groups,
    storage::{Animation, Asset},
    ui::{
        asset_tree::{AssetTree, TreeSelection, TREE_WIDTH},
        effect::EffectWidget,
    },
    wgpu_render_state,
};
use egui::{Context, Margin, Vec2};
use std::iter::once;
use wgpu::CommandEncoderDescriptor;

#[derive(Default)]
pub struct AnimationWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Animation>,
    effect: Option<Effect>,
    effect_state: Option<EffectState>,
    preview: bool,
}

impl AnimationWindow {
    pub fn update(&mut self, ctx: &Context, timing: &Timing) {
        if !self.open {
            self.dirty = false;
            return;
        }

        if self.preview {
            self.show_preview_window(ctx, timing);
        }

        egui::Window::new("Animations")
            .collapsible(false)
            .min_width(1000.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::SidePanel::left("animations tree")
                    .exact_width(TREE_WIDTH)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        if self.tree.show(ui, ui.make_persistent_id("animations_tree")) {
                            self.dirty = false;

                            if let TreeSelection::Asset(animation) = &self.tree.selected() {
                                let effect = Effect {
                                    animation: Some(animation.id),
                                    ..Default::default()
                                };
                                self.effect_state = Some(EffectState::new(&effect));
                                self.effect = Some(effect);
                            }
                        }
                    });

                egui::SidePanel::right("scene editor")
                    .exact_width(300.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        if let TreeSelection::Asset(animation) = &mut self.tree.selected() {
                            ui.vertical_centered_justified(|ui| {
                                if ui.button("👁 Open Preview").clicked() {
                                    self.preview = true;
                                }
                            });
                            self.dirty |= animation.data.change_arguments_ui(ui);
                        }
                    });

                egui::Frame::default()
                    .outer_margin(Margin::same(4.0))
                    .show(ui, |ui| {
                        if self.tree.common_settings(ui, &mut self.dirty) {
                            if let (Some(effect), Some(effect_state)) =
                                (&mut self.effect, &mut self.effect_state)
                            {
                                effect_state.update(effect);
                            };
                        }

                        if let TreeSelection::Asset(animation) = &mut self.tree.selected() {
                            self.dirty |= animation.data.change_shader_code_ui(ui);
                        }
                    });
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn show_preview_window(&mut self, ctx: &Context, timing: &Timing) {
        let (Some(effect), Some(effect_state)) = (&mut self.effect, &mut self.effect_state) else {
            return;
        };

        {
            effect_state.beat_progression = timing.beat_progression();
            effect_state.beats_per_minute = timing.beats_per_minute;
            effect_state.framerate = timing.framerate().unwrap_or_default();
            let wgpu_render_state = wgpu_render_state();
            let device = wgpu_render_state.device;
            let queue = &wgpu_render_state.queue;
            effect.prepare(
                effect_state,
                queue,
                PersistantState::get().preview_palette.and_then(Asset::get),
                &Groups::None,
                1.0,
            );
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render animations for scene editor"),
            });
            effect.render(effect_state, &mut encoder, false);
            queue.submit(once(encoder.finish()));
        }

        egui::Window::new("Animation Preview")
            .collapsible(false)
            .min_width(600.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.preview)
            .show(ctx, |ui| {
                egui::SidePanel::right("animation preview right side")
                    .exact_width(300.0)
                    .resizable(false)
                    .show_inside(ui, |ui| effect.config_ui(effect_state, ui, false));

                egui::Frame::default()
                    .outer_margin(Margin::same(4.0))
                    .show(ui, |ui| {
                        ui.add_sized(
                            Vec2::splat(300.0),
                            EffectWidget {
                                show_group: false,
                                selectable: None,
                                effect,
                                effect_state,
                            },
                        );
                    });
            });
    }
}
