use super::{widget::EffectWidget, App};
use crate::{
    app::PersistantState,
    transition::{Transition, TransitionGoal},
};
use egui::{scroll_area::ScrollBarVisibility, Color32, Rect, TextureId, Ui, Vec2};

impl App {
    pub fn effects_grid(&mut self, ui: &mut Ui, svg: Option<TextureId>, uv: Option<Rect>) {
        let effects_size = PersistantState::effects_size();
        let effects_show_svg = PersistantState::effects_show_svg();

        egui::ScrollArea::vertical()
            .id_salt("effects_scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 30.0);
                ui.horizontal_wrapped(|ui| {
                    let mut changed = None;
                    let mut flashed = Vec::new();
                    for (index, effect) in self.pipeline.effects().into_iter() {
                        let response = ui.add_sized(
                            Vec2::new(effects_size + 40.0, effects_size + 60.0),
                            EffectWidget {
                                selected_effect: &mut self.selected_effect,
                                hovered_effect: &mut self.hovered_effect,
                                index,
                                effect,
                                svg: svg.filter(|_| effects_show_svg),
                                effects_size,
                                live_color: Color32::GREEN,
                                uv,
                            },
                        );
                        if response.changed()
                            || effect
                                .selection_input
                                .as_ref()
                                .map(|event| event.is_new())
                                .unwrap_or_default()
                        {
                            changed = Some(index);
                        }

                        if let Some(event) = effect.flash_input.as_ref() {
                            if event.is_live() {
                                flashed.push(index);
                            }
                        }

                        if let Some(event) = effect.dimmer_input.as_ref() {
                            effect.set_input_dimmer(event.dimmer());
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
