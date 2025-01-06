use crate::{app::timing::Timing, storage::asset::project::Project};
use egui::Context;

pub mod about;
pub mod animation;
pub mod artnet_input;
pub mod curves;
pub mod errors;
pub mod git_config;
pub mod output_devices;
pub mod output_routings;
pub mod palettes;
pub mod projects;
pub mod scenes;
pub mod shortcuts;

#[derive(Default)]
pub struct Windows {
    pub about: about::AboutWindow,
    pub animations: animation::AnimationWindow,
    pub artnet_input: artnet_input::ArtnetInputWindow,
    pub errors: errors::ErrorsWindow,
    pub curves: curves::CurvesWindow,
    pub git_config: git_config::GitConfigWindow,
    pub output_devices: output_devices::OutputDevicesWindow,
    pub output_routings: output_routings::OutputRoutingsWindow,
    pub palettes: palettes::PalettesWindow,
    pub projects: projects::ProjectsWindow,
    pub scenes: scenes::ScenesWindow,
    pub shortcuts: shortcuts::ShortcutsWindow,
}

impl Windows {
    pub fn update(&mut self, ctx: &Context, timing: &Timing, project: Option<&mut Project>) {
        self.about.update(ctx);
        self.animations.update(ctx, timing);
        self.curves.update(ctx);
        self.errors.update(ctx);
        self.git_config.update(ctx);
        self.artnet_input.update(ctx);
        self.output_devices.update(ctx);
        self.output_routings.update(ctx);
        self.palettes.update(ctx);
        self.projects.update(ctx);
        self.scenes.update(ctx, timing);
        self.shortcuts.update(ctx, project);
    }
}
