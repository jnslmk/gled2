use super::App;
use crate::{
    app::PersistantState,
    transition::{Transition, TransitionGoal},
    ui::scene_instance::widget::SceneInstanceWidget,
};
use egui::{scroll_area::ScrollBarVisibility, Color32, Rect, TextureId, Ui, Vec2};

impl App {
    pub fn effects_grid(&mut self, ui: &mut Ui, svg: Option<TextureId>, uv: Option<Rect>) {
        let Some(project) = self.project.as_mut() else {
            return;
        };

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
                    for (path, scene_instance) in project.all_scene_instances() {
                        let response = ui.add_sized(
                            Vec2::new(effects_size + 40.0, effects_size + 60.0),
                            SceneInstanceWidget {
                                selected_scene_instance: &mut self.selected_scene_instance,
                                hovered_scene_instance: &mut self.hovered_scene_instance,
                                path,
                                scene_instance,
                                svg: svg.filter(|_| effects_show_svg),
                                effects_size,
                                live_color: Color32::GREEN,
                                uv,
                            },
                        );
                        if response.changed()
                            || scene_instance
                                .selection_input
                                .as_ref()
                                .map(|event| event.is_new())
                                .unwrap_or_default()
                        {
                            changed = Some(path);
                        }

                        if let Some(event) = scene_instance.flash_input.as_ref() {
                            if event.is_live() {
                                flashed.push(path);
                            }
                        }

                        if let Some(event) = scene_instance.dimmer_input.as_ref() {
                            scene_instance.set_input_dimmer(event.dimmer());
                        }
                    }

                    if let Some(changed_path) = changed {
                        if let Some(scene_instance) = project.scene_instance(changed_path) {
                            scene_instance.set_transition(Transition::new(
                                if scene_instance.active {
                                    TransitionGoal::TurnOff
                                } else {
                                    TransitionGoal::TurnOn
                                },
                                self.timing.fade_duration(),
                            ));
                        }
                    }

                    for (path, scene_instance) in project.all_scene_instances() {
                        scene_instance.set_flash(flashed.contains(&path));
                    }
                });
            });
    }
}
