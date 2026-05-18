use crate::{
    app::{persistent_state::PersistentState, timing::Timing},
    audio::sound_data::SoundData,
    pipeline::extract_output::ExtractOutput,
    storage::{asset::project::Project, collections::Collections},
};
use egui::Context;

pub mod about;
pub mod animation;
pub mod channel_overwrites;
pub mod curves;
pub mod errors;
pub mod external_devices;
pub mod git_config;
pub mod midi_controllers;
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
    pub external_device_settings: external_devices::ExternalDeviceSettings,
    pub channel_overwrites: channel_overwrites::ChannelOverwritesWindow,
    pub curves: curves::CurvesWindow,
    pub errors: errors::ErrorsWindow,
    pub git_config: git_config::GitConfigWindow,
    pub midi_controllers: midi_controllers::MidiControllersWindow,
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
        project: &mut Option<Project>,
        collections: &mut Collections,
        persistent_state: &mut PersistentState,
        extract_output: &mut ExtractOutput,
        sound_data: &mut SoundData,
        midi_monitor_receiver: &kanal::Receiver<crate::midi::monitor::MidiMonitorEvent>,
        test_command_sender: &kanal::Sender<crate::midi::runtime::TestCommand>,
        midi_learn_state: &mut crate::midi::learn::LearnState,
    ) {
        self.about.update(ctx);
        self.animations
            .update(ctx, timing, collections, persistent_state, sound_data);
        self.channel_overwrites.update(ctx, collections);
        self.curves.update(ctx, collections);
        self.errors.update(ctx);
        self.git_config.update(ctx, persistent_state);
        self.midi_controllers.update(
            ctx,
            project,
            collections,
            midi_monitor_receiver,
            test_command_sender,
            midi_learn_state,
        );
        let midi_diagnostics = self.midi_controllers.diagnostics();
        self.external_device_settings
            .update(ctx, project, collections, &midi_diagnostics);
        self.output_devices.update(ctx, collections);
        self.output_routings
            .update(ctx, collections, extract_output);
        self.palettes.update(ctx, collections);
        self.projects.update(ctx, collections);
        self.scenes
            .update(ctx, timing, collections, persistent_state, sound_data);
        self.shortcuts.update(ctx, project);
    }
}
