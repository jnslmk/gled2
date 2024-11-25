use crate::{app::Timing, storage::Project};
use egui::Context;

mod about;
mod animation;
mod artnet_input;
mod curves;
mod output_devices;
mod output_routings;
mod palettes;
mod projects;
mod scenes;
mod shortcuts;

#[derive(Default)]
pub struct Windows {
    pub about: about::AboutWindow,
    pub animations: animation::AnimationWindow,
    pub artnet_input: artnet_input::ArtnetInputWindow,
    pub curves: curves::CurvesWindow,
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
        self.artnet_input.update(ctx);
        self.output_devices.update(ctx);
        self.output_routings.update(ctx);
        self.palettes.update(ctx);
        self.projects.update(ctx);
        self.scenes.update(ctx, timing);
        self.shortcuts.update(ctx, project);
    }
}
