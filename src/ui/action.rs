use crate::{
    app::{App, persistant_state::PersistantState, svg::Svg},
    input::artnet::ARTNET_CONFIG,
    pipeline::extract_output::ExtractOutput,
    storage::{
        asset::{
            Asset,
            animation::Animation,
            curve::multiplied_curve::MultipliedCurve,
            project::{Project, scene_instance_path::SceneInstancePathId},
        },
        asset_id::AssetId,
    },
};
use egui::ViewportId;
use notify_rust::Notification;
use once_cell::sync::OnceCell;
use std::sync::{
    Arc,
    mpsc::{Receiver, Sender},
};
use crate::storage::asset::project::scene_instance_path::{SceneInstanceUnion};

static ACTION_SENDER: OnceCell<Sender<UiAction>> = OnceCell::new();

#[derive(Debug)]
pub enum UiAction {
    SetProject(AssetId<Project>),
    SelectScene(SceneInstanceUnion),
    DeleteSelectedSceneInstance,
    CloneSelectedSceneInstance,
    InitGPU,
    SendPositions,
    ReloadShaderCode(AssetId<Animation>),
    CloseWindow(ViewportId),
    SetSvg(Option<Svg>),
    OpenGitConfigWindow,
    Tap,
    SpeedAdd(f32),
    SpeedMultiply(f32),
    SetBlackout(bool),
    /// Set opacity for a scene instance
    ///
    /// This also activates the scene instance
    SetSceneOpacity(SceneInstanceUnion, f32),
    SetSelectedSceneOpacity(f32),
    ToggleSceneActive(SceneInstanceUnion),
    SetSceneActive(SceneInstanceUnion, bool),
    SetMainDimmer(f32),
    MidiOutputActive(bool),
    Error(String),
}

impl App {
    pub fn handle_ui_actions(&mut self) {
        loop {
            let Some(action) = self.ui_action_receiver.try_recv().ok() else {
                return;
            };
            log::trace!("Handling ui action: {action:?}");

            match (&mut self.project, action) {
                (Some(project), UiAction::DeleteSelectedSceneInstance) => {
                    project.remove_scene_instance(&mut self.selected_scene_instance);
                    UiAction::InitGPU.enqueue();
                }
                (Some(project), UiAction::CloneSelectedSceneInstance) => {
                    if let Some(scene) = project
                        .scene_instance_mut(self.selected_scene_instance)
                        .map(|scene_instance| scene_instance.scene)
                    {
                        self.selected_scene_instance = project.add_scene(scene);
                        UiAction::InitGPU.enqueue();
                    }
                }
                (Some(project), UiAction::InitGPU) => {
                    project.init_gpu();
                }
                (Some(project), UiAction::ReloadShaderCode(animation)) => {
                    project.reload_shader_code(animation);
                    self.windows.scenes.reload_shader_code(animation);
                }
                (Some(project), UiAction::SendPositions) => {
                    project.send_positions();
                }
                (Some(project), UiAction::SetSvg(svg)) => {
                    project.svg = svg;
                    project.remove_nonexistant_groups();
                }
                (Some(project), UiAction::SetSceneOpacity(path, opacity)) => {
                    if let Some(scene_instance) = project.scene_instance_by_index_or_quick(path) {
                        scene_instance.opacity = MultipliedCurve::new_multiplier(opacity);
                    }
                }
                (Some(project), UiAction::SetSelectedSceneOpacity(opacity)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_mut(self.selected_scene_instance)
                    {
                        scene_instance.opacity = MultipliedCurve::new_multiplier(opacity);
                    }
                }
                (Some(project), UiAction::SetMainDimmer(dimmer)) => {
                    project.main_dimmer = dimmer;
                }
                (Some(project), UiAction::ToggleSceneActive(location)) => {
                    if let Some(scene_instance) = project.scene_instance_by_index_or_quick(location) {
                        scene_instance.active = !scene_instance.active;
                    }
                }
                (Some(project), UiAction::SetSceneActive(location, active)) => {
                    if let Some(scene_instance) = project.scene_instance_by_index_or_quick(location) {
                        scene_instance.active = active;
                    }
                }
                (Some(project), UiAction::SelectScene(location)) => {
                    if let Some(scene_instance) = project.scene_instance_by_index_or_quick(location) {
                        self.selected_scene_instance = SceneInstancePathId {
                            id: scene_instance.id,
                        };
                    }
                }
                (_, UiAction::Tap) => {
                    self.timing.tap();
                }
                (_, UiAction::SpeedAdd(delta)) => {
                    self.timing.add_speed(delta);
                }
                (_, UiAction::SpeedMultiply(multiplier)) => {
                    self.timing.multiply_speed(multiplier);
                }
                (_, UiAction::SetBlackout(blackout)) => {
                    self.blackout = blackout;
                }
                (_, UiAction::SetProject(project)) => {
                    if let Some(project) = Asset::get(project) {
                        let mut persistant_state = PersistantState::get();
                        persistant_state.last_project_id = Some(project.id);
                        persistant_state.save();

                        self.project_id = Some(project.id);
                        let mut project = Arc::unwrap_or_clone(project).data;
                        self.selected_scene_instance = project
                            .all_scene_instances_id()
                            .next()
                            .map(|(path, _)| path)
                            .unwrap_or_default();
                        *ExtractOutput::get().routings.lock() = project.output_routings.clone();
                        *ARTNET_CONFIG.lock() = project.artnet_config.clone();
                        project.channel_overwrites.clone().set();
                        self.project = Some(project);
                    } else {
                        self.windows.artnet_input.close();
                        self.windows.output_routings.close();
                        self.windows.shortcuts.close();
                        self.project.take();
                        self.project_id.take();
                    };
                    UiAction::InitGPU.enqueue();
                    Svg::reset();
                }
                (_, UiAction::CloseWindow(viewport_id)) => {
                    self.other_main_windows.remove(&viewport_id);
                }
                (_, UiAction::OpenGitConfigWindow) => {
                    self.windows.git_config.open();
                }
                (_, UiAction::MidiOutputActive(active)) => {
                    self.midi_output_active = active;
                }
                (_, UiAction::Error(error)) => {
                    log::error!("{error}");

                    if let Err(err) = Notification::new()
                        .summary("Error")
                        .body(&error)
                        .icon("application-gled")
                        .show()
                    {
                        log::error!("Could not show notification: {err:?}");
                    }

                    self.windows.errors.entries.push(error);
                }
                (None, _) => log::trace!("Ingoring ui action which needs a loaded project"),
            }
        }
    }
}

impl UiAction {
    pub fn init_queue() -> Receiver<UiAction> {
        let (sender, receiver) = std::sync::mpsc::channel();
        ACTION_SENDER.set(sender).unwrap();
        receiver
    }

    pub fn enqueue(self) {
        ACTION_SENDER
            .get()
            .expect("Action sender not initialized")
            .send(self)
            .expect("Action receiver dropped");
    }
}
