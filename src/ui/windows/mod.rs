use crate::{
    app::{persistant_state::PersistantState, timing::Timing},
    storage::{asset::project::Project, collections::Collections},
};
use egui::Context;

pub mod about;
pub mod animation;
pub mod artnet_input;
pub mod channel_overwrites;
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
    pub channel_overwrites: channel_overwrites::ChannelOverwritesWindow,
    pub curves: curves::CurvesWindow,
    pub errors: errors::ErrorsWindow,
    pub git_config: git_config::GitConfigWindow,
    pub output_devices: output_devices::OutputDevicesWindow,
    pub output_routings: output_routings::OutputRoutingsWindow,
    pub palettes: palettes::PalettesWindow,
    pub projects: projects::ProjectsWindow,
    pub scenes: scenes::ScenesWindow,
    pub shortcuts: shortcuts::ShortcutsWindow,
}

impl Windows {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(
        &mut self,
        ctx: &Context,
        timing: &Timing,
        project: Option<&mut Project>,
        collections: &mut Collections,
        persistant_state: &mut PersistantState,
    ) {
        self.about.update(ctx);
        self.animations
            .update(ctx, timing, collections, persistant_state);
        self.channel_overwrites.update(ctx, collections);
        self.curves.update(ctx, collections);
        self.errors.update(ctx);
        self.git_config.update(ctx, persistant_state);
        self.artnet_input.update(ctx, collections);
        self.output_devices.update(ctx, collections);
        self.output_routings.update(ctx, collections);
        self.palettes.update(ctx, collections);
        self.projects.update(ctx, collections);
        self.scenes
            .update(ctx, timing, collections, persistant_state);
        self.shortcuts.update(ctx, project);
    }
}
