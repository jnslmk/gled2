use crate::{
    app::{persistant_state::PersistantState, svg::Svg, App},
    input::artnet::ARTNET_CONFIG,
    pipeline::extract_output::ExtractOutput,
    storage::{
        asset::{
            animation::Animation,
            curve::static_or_curve::StaticOrCurve,
            project::{scene_instance_path::SceneInstancePath, Project},
            Asset,
        },
        asset_id::AssetId,
    },
};
use egui::ViewportId;
use once_cell::sync::OnceCell;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

static ACTION_SENDER: OnceCell<Sender<UiAction>> = OnceCell::new();

#[derive(Debug)]
pub enum UiAction {
    SetProject(AssetId<Project>),
    DeleteSelectedSceneInstance,
    CloneSelectedSceneInstance,
    InitGPU,
    SendPositions,
    ReloadShaderCode(AssetId<Animation>),
    CloseWindow(ViewportId),
    SetSvg(Option<Svg>),
    OpenGitConfigWindow,
    SetCrossFader(f32),
    Tap,
    SpeedAdd(f32),
    SpeedMultiply(f32),
    SetBlackout(bool),
    /// Set opacity for a scene instance
    ///
    /// This also activates the scene instance
    SetSceneOpacity(SceneInstancePath, f32),
    ToggleSceneActive(SceneInstancePath),
    SetSceneActive(SceneInstancePath, bool),
    SetMainDimmer(f32),
    MidiOutputActive(bool),
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
                        .scene_instance(self.selected_scene_instance)
                        .map(|scene_instance| scene_instance.scene)
                    {
                        project
                            .deck(self.selected_scene_instance)
                            .add_scene(&mut self.selected_scene_instance, scene);
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
                (Some(project), UiAction::SetCrossFader(fader)) => {
                    project.cross_fader = fader;
                }
                (Some(project), UiAction::SetSceneOpacity(path, opacity)) => {
                    if let Some(scene_instance) = project.scene_instance(path) {
                        scene_instance.opacity = StaticOrCurve::new_static(opacity);
                    }
                }
                (Some(project), UiAction::SetMainDimmer(dimmer)) => {
                    project.main_dimmer = dimmer;
                }
                (Some(project), UiAction::ToggleSceneActive(path)) => {
                    if let Some(scene_instance) = project.scene_instance(path) {
                        scene_instance.active = !scene_instance.active;
                    }
                }
                (Some(project), UiAction::SetSceneActive(path, active)) => {
                    if let Some(scene_instance) = project.scene_instance(path) {
                        scene_instance.active = active;
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
                        let project = Arc::unwrap_or_clone(project).data;
                        *ExtractOutput::get().routings.lock() = project.output_routings.clone();
                        *ARTNET_CONFIG.lock() = project.artnet_config.clone();
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
                    self.selected_scene_instance = SceneInstancePath::default();
                    self.hovered_scene_instance = SceneInstancePath::default();
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
        if let Some(sender) = ACTION_SENDER.get() {
            sender.send(self).expect("Action receiver dropped");
        }
    }
}
