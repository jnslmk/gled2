use crate::storage::asset::project::scene_instance_path::SceneInstanceUnion;
use crate::storage::asset::scene::Scene;
use crate::storage::asset::scene::grid::GridLocation;
use crate::{
    app::{App, svg::Svg},
    input::artnet::ARTNET_CONFIG,
    pipeline::extract_output::ExtractOutput,
    storage::{
        asset::{
            Asset, animation::Animation, curve::multiplied_curve::MultipliedCurve, project::Project,
        },
        asset_id::AssetId,
    },
};
use egui::ViewportId;
use kanal::{Receiver, Sender, unbounded};
use notify_rust::Notification;
use once_cell::sync::OnceCell;
use std::sync::Arc;

static ACTION_SENDER: OnceCell<Sender<UiAction>> = OnceCell::new();

#[derive(Debug)]
pub enum UiAction {
    AddScene(GridLocation, AssetId<Scene>),
    SetProject(AssetId<Project>),
    SelectScene(SceneInstanceUnion),
    SelectSceneByLocation(GridLocation),
    DeleteSceneInstance {
        location: GridLocation,
    },
    DeleteSelectedSceneInstance,
    CloneSelectedSceneInstance,
    CloneSceneInstance(GridLocation),
    SendPositions,
    ReloadShaderCode(Option<AssetId<Animation>>),
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
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn handle_ui_actions(&mut self) {
        loop {
            let Ok(Some(action)) = self.ui_action_receiver.try_recv() else {
                return;
            };
            log::trace!("Handling ui action: {action:?}");

            match (&mut self.project, action) {
                (Some(project), UiAction::AddScene(pos, scene)) => {
                    project.add_scene(pos, scene, &self.collections);
                }
                (Some(project), UiAction::DeleteSceneInstance { location }) => {
                    project.remove_scene_instance(location);
                }
                (Some(project), UiAction::DeleteSelectedSceneInstance) => {
                    project.remove_scene_instance(self.selected_scene_instance);
                }

                (Some(project), UiAction::CloneSelectedSceneInstance) => {
                    if let Some(scene_id) = project
                        .get_scenes_instance(&self.selected_scene_instance)
                        .map(|scene_instance| scene_instance.scene_id)
                    {
                        project.add_scene(
                            project.next_empty_grid_location(self.selected_scene_instance),
                            scene_id,
                            &self.collections,
                        );
                    }
                }
                (Some(project), UiAction::CloneSceneInstance(location)) => {
                    if let Some(scene_id) = project
                        .get_scenes_instance(&location)
                        .map(|scene_instance| scene_instance.scene_id)
                    {
                        project.add_scene(
                            project.next_empty_grid_location(location),
                            scene_id,
                            &self.collections,
                        );
                    }
                }
                (Some(project), UiAction::ReloadShaderCode(animation)) => {
                    project.reload_shader_code(animation, &self.collections);
                    self.windows
                        .scenes
                        .reload_shader_code(animation, &mut self.collections);
                }
                (Some(project), UiAction::SendPositions) => {
                    project.send_positions();
                }
                (Some(project), UiAction::SetSvg(svg)) => {
                    project.svg = svg;
                    project.remove_nonexistant_groups();
                }
                (Some(project), UiAction::SetSceneOpacity(path, opacity)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.opacity = MultipliedCurve::new_multiplier(opacity);
                    }
                }
                (Some(project), UiAction::SetSelectedSceneOpacity(opacity)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.opacity = MultipliedCurve::new_multiplier(opacity);
                    }
                }
                (Some(project), UiAction::SetMainDimmer(dimmer)) => {
                    project.main_dimmer = dimmer;
                }
                (Some(project), UiAction::ToggleSceneActive(location)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(location)
                    {
                        scene_instance.active = !scene_instance.active;
                    }
                }
                (Some(project), UiAction::SetSceneActive(location, active)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(location)
                    {
                        scene_instance.active = active;
                    }
                }
                (Some(project), UiAction::SelectScene(location)) => {
                    if let Some(pos) = project.location_by_location_or_quick_index(location) {
                        self.selected_scene_instance = pos;
                    }
                }
                (Some(project), UiAction::SelectSceneByLocation(location)) => {
                    if project.scenes_instances_grid.contains_key(&location) {
                        self.selected_scene_instance = location;
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
                    if let Some(project) = Asset::get(project, &self.collections) {
                        self.persistant_state.set_last_project_id(project.id);
                        self.persistant_state.save();

                        self.project_id = Some(project.id);
                        let project = Arc::unwrap_or_clone(project).data;
                        self.selected_scene_instance = project
                            .all_scene_instance_locations()
                            .next()
                            .copied()
                            .unwrap_or_default();
                        *ExtractOutput::get().routings.lock() = project.output_routings.clone();
                        *ARTNET_CONFIG.lock() = project.artnet_config.clone();
                        project.channel_overwrites.clone().set();
                        self.project = Some(project);
                        UiAction::ReloadShaderCode(None).enqueue();
                    } else {
                        self.windows.artnet_input.close();
                        self.windows.output_routings.close();
                        self.windows.shortcuts.close();
                        self.project.take();
                        self.project_id.take();
                    };
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
        let (sender, receiver) = unbounded();
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
