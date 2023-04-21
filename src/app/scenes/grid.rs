use super::{widget::SceneWidget, App};
use crate::{
    scene::SceneKind,
    transition::{Transition, TransitionGoal},
};
use egui::{Color32, Context, Rect, TextureId, Ui, Vec2};

impl App {
    pub fn scenes_grid(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        kind: SceneKind,
        svg: Option<TextureId>,
        uv: Option<Rect>,
    ) {
        let scenes = match kind {
            SceneKind::Background => &self.persistant_state.background,
            SceneKind::Foreground => &self.persistant_state.foreground,
        };

        egui::ScrollArea::vertical()
            .id_source(format!("{kind:?}_scroll"))
            .auto_shrink([false, false])
            .always_show_scroll(true)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 30.0);
                ui.horizontal_wrapped(|ui| {
                    let mut changed = None;
                    let mut flashed = Vec::new();
                    for (index, scene) in self
                        .pipeline
                        .scenes()
                        .into_iter()
                        .filter(|(_index, scene)| scene.kind == kind)
                    {
                        let response = ui.add_sized(
                            Vec2::new(scenes.size + 40.0, scenes.size + 60.0),
                            SceneWidget {
                                selected_scene: &mut self.selected_scene,
                                hovered_scene: &mut self.hovered_scene,
                                index,
                                scene,
                                svg: svg.filter(|_| scenes.show_svg),
                                scenes_size: scenes.size,
                                live_color: match kind {
                                    SceneKind::Background => Color32::DARK_BLUE,
                                    SceneKind::Foreground => Color32::DARK_RED,
                                },
                                uv,
                            },
                        );
                        if response.changed()
                            || scene
                                .hotkey
                                .as_ref()
                                .map(|hotkey| hotkey.pressed(ctx, &self.gamepad))
                                .unwrap_or_default()
                        {
                            changed = Some(index);
                        }

                        if scene
                            .flash_hotkey
                            .as_ref()
                            .map(|hotkey| hotkey.live(ctx, &self.gamepad))
                            .unwrap_or_default()
                        {
                            flashed.push(index);
                        }
                    }

                    if let Some(changed_index) = changed {
                        if let Some(scene) = self.pipeline.scene(changed_index) {
                            scene.set_transition(Transition::new(
                                if scene.active {
                                    TransitionGoal::TurnOff
                                } else {
                                    TransitionGoal::TurnOn
                                },
                                self.timing.fade_duration(),
                            ));
                        }

                        if kind == SceneKind::Foreground {
                            for (_index, scene) in
                                self.pipeline.scenes().into_iter().filter(|(index, scene)| {
                                    scene.kind == SceneKind::Foreground
                                        && *index != changed_index
                                        && scene.on()
                                })
                            {
                                scene.set_transition(Transition::new(
                                    TransitionGoal::TurnOff,
                                    self.timing.fade_duration(),
                                ));
                            }
                        }
                    }

                    for (index, scene) in self
                        .pipeline
                        .scenes()
                        .iter_mut()
                        .filter(|(_index, scene)| scene.kind == kind)
                    {
                        scene.set_flash(flashed.contains(index));
                    }
                });
            });
    }
}
