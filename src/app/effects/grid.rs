use super::{widget::EffectWidget, App};
use crate::transition::{Transition, TransitionGoal};
use egui::{scroll_area::ScrollBarVisibility, Color32, Context, Rect, TextureId, Ui, Vec2};

impl App {
    pub fn effects_grid(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        svg: Option<TextureId>,
        uv: Option<Rect>,
    ) {
        let effects = &self.persistant_state.effects;

        egui::ScrollArea::vertical()
            .id_source("effects_scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 30.0);
                ui.horizontal_wrapped(|ui| {
                    let mut changed = None;
                    let mut flashed = Vec::new();
                    for (index, effect) in self.pipeline.effects().into_iter() {
                        let response = ui.add_sized(
                            Vec2::new(effects.size + 40.0, effects.size + 60.0),
                            EffectWidget {
                                selected_effect: &mut self.selected_effect,
                                hovered_effect: &mut self.hovered_effect,
                                index,
                                effect,
                                svg: svg.filter(|_| effects.show_svg),
                                effects_size: effects.size,
                                live_color: Color32::GREEN,
                                uv,
                            },
                        );
                        if response.changed()
                            || effect
                                .hotkey
                                .as_ref()
                                .map(|hotkey| hotkey.pressed(ctx, &self.gamepad))
                                .unwrap_or_default()
                        {
                            changed = Some(index);
                        }

                        if let Some(hotkey) = effect.flash_hotkey.as_ref() {
                            if hotkey.live(ctx, &self.gamepad) {
                                flashed.push(index);
                            }

                            effect.set_hotkey_dimmer(hotkey.dimmer(&self.gamepad));
                        }
                    }

                    if let Some(changed_index) = changed {
                        if let Some(effect) = self.pipeline.effect(changed_index) {
                            effect.set_transition(Transition::new(
                                if effect.active {
                                    TransitionGoal::TurnOff
                                } else {
                                    TransitionGoal::TurnOn
                                },
                                self.timing.fade_duration(),
                            ));
                        }
                    }

                    for (index, effect) in self.pipeline.effects().iter_mut() {
                        effect.set_flash(flashed.contains(index));
                    }
                });
            });
    }
}
