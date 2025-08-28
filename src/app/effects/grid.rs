use super::App;
use crate::{
    app::PersistantState,
    pipeline::transition::{Transition, TransitionGoal},
    storage::asset::project::scene_instance_path::SceneInstancePath,
    ui::scene_instance::widget::SceneInstanceWidget,
};
use egui::{Color32, Rect, TextureHandle, Ui, Vec2, scroll_area::ScrollBarVisibility};

impl App {
    pub fn effects_grid(&mut self, ui: &mut Ui, svg: Option<TextureHandle>, uv: Option<Rect>) {
        egui::ScrollArea::vertical()
            .id_salt("grid scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 30.0);
                ui.horizontal_wrapped(|ui| {
                    self.widgets(ui, svg, uv, SceneInstancePath::GRID);
                });
            });
    }

    pub fn effects_quick(&mut self, ui: &mut Ui, svg: Option<TextureHandle>, uv: Option<Rect>) {
        egui::ScrollArea::horizontal()
            .id_salt("quick scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .vscroll(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    self.widgets(ui, svg, uv, SceneInstancePath::QUICK);
                });
            });
    }

    fn widgets(
        &mut self,
        ui: &mut Ui,
        svg: Option<TextureHandle>,
        uv: Option<Rect>,
        path: SceneInstancePath,
    ) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let effects_size = PersistantState::effects_size();
        let mut changed = None;
        let groups = project.groups.clone();
        for (path, scene_instance) in project.scene_instances(path) {
            let response = ui.add_sized(
                Vec2::new(effects_size + 40.0, effects_size + 60.0),
                SceneInstanceWidget {
                    selected_scene_instance: &mut self.selected_scene_instance,
                    hovered_scene_instance: &mut self.hovered_scene_instance,
                    path,
                    scene_instance,
                    svg: svg.clone(),
                    effects_size,
                    live_color: Color32::DARK_GREEN,
                    uv,
                    groups: &groups,
                },
            );
            if response.changed()
                || scene_instance
                    .activation_input
                    .as_ref()
                    .map(|event| event.is_new())
                    .unwrap_or_default()
            {
                changed = Some(path);
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
    }
}
