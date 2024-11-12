mod about;
mod artnet_input;
mod output_devices;
mod output_routings;
mod palettes;
mod scenes;
mod shortcuts;

#[derive(Default)]
pub struct Windows {
    pub about: about::AboutWindow,
    pub artnet_input: artnet_input::ArtnetInputWindow,
    pub output_devices: output_devices::OutputDevicesWindow,
    pub output_routings: output_routings::OutputRoutingsWindow,
    pub palettes: palettes::PalettesWindow,
    pub scenes: scenes::ScenesWindow,
    pub shortcuts: shortcuts::ShortcutsWindow,
}

impl Windows {
    pub fn update(&mut self, ctx: &egui::Context) {
        self.about.update(ctx);
        self.artnet_input.update(ctx);
        self.output_devices.update(ctx);
        self.output_routings.update(ctx);
        self.palettes.update(ctx);
        self.scenes.update(ctx);
        self.shortcuts.update(ctx);
    }
}
