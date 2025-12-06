use super::App;
use crate::{
    app::PersistantState,
    pipeline::transition::{Transition, TransitionGoal},
    storage::asset::project::{DeckPath, scene_instance_path::SceneInstancePathId},
    ui::scene_instance::widget::SceneInstanceWidget,
};
use egui::{TextureHandle, Ui, Vec2, scroll_area::ScrollBarVisibility::AlwaysVisible};
use egui_dnd::dnd;

impl App {
    pub fn effects_grid(&mut self, ui: &mut Ui, svg: Option<TextureHandle>) {
        egui::ScrollArea::vertical()
            .id_salt("grid scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(AlwaysVisible)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 30.0);
                ui.horizontal_wrapped(|ui| {
                    self.widgets(ui, svg, DeckPath::Grid);
                });
            });
    }

    pub fn effects_quick(&mut self, ui: &mut Ui, svg: Option<TextureHandle>) {
        egui::ScrollArea::horizontal()
            .id_salt("quick scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(AlwaysVisible)
            .vscroll(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    self.widgets(ui, svg, DeckPath::Quick);
                });
            });
    }

    fn widgets(&mut self, ui: &mut Ui, svg: Option<TextureHandle>, deck_path: DeckPath) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let effects_size = PersistantState::effects_size();
        let groups = project.groups.clone();

        let size = Vec2::splat(effects_size);
        let scene_instances = project.scene_instances(deck_path);
        let response = dnd(ui, deck_path).show_sized(
            scene_instances.iter_mut(),
            size + Vec2::new(60.0, 60.0),
            |ui, scene_instance, dnd_handle, state| {
                if state.dragged {
                    self.selected_scene_instance =
                        SceneInstancePathId::new(deck_path, scene_instance.id);
                }

                let response = ui.add(SceneInstanceWidget {
                    selected_scene_instance: &mut self.selected_scene_instance,
                    deck_path,
                    scene_instance,
                    dnd_handle,
                    svg: svg.clone(),
                    size,
                    groups: &groups,
                    timing: &self.timing,
                });
                if response.changed()
                    || scene_instance
                        .activation_input
                        .as_ref()
                        .map(|event| event.is_new())
                        .unwrap_or_default()
                {
                    scene_instance.set_transition(Transition::new(
                        if scene_instance.active {
                            TransitionGoal::TurnOff
                        } else {
                            TransitionGoal::TurnOn
                        },
                        self.timing.fade_duration(),
                    ));
                }
            },
        );

        if response.is_drag_finished() {
            response.update_vec(scene_instances);
        }
    }
}
