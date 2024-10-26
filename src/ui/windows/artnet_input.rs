use crate::input::ARTNET_INPUT_CONFIG;
use egui::{Context, TextEdit};

#[derive(Default)]
pub struct ArtnetInputWindow {
    open: bool,
    universe: Option<String>,
    start: Option<String>,
    channels: Option<String>,
}

impl ArtnetInputWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        egui::Window::new("Config Artnet Inputs")
            .collapsible(false)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                let mut config = ARTNET_INPUT_CONFIG.lock();

                ui.label("Universe");
                let universe = self
                    .universe
                    .get_or_insert_with(|| config.universe.to_string());
                if ui.add(TextEdit::singleline(universe)).changed()
                    && universe.parse::<u16>().is_ok()
                {
                    config.universe = universe.parse().expect("Should never happen");
                }

                ui.label("Start channel");
                let start = self.start.get_or_insert_with(|| config.start.to_string());
                if ui.add(TextEdit::singleline(start)).changed() && start.parse::<u16>().is_ok() {
                    config.start = start.parse().expect("Should never happen");
                }

                ui.label("Channels");
                let channels = self
                    .channels
                    .get_or_insert_with(|| config.channels.to_string());
                if ui.add(TextEdit::singleline(channels)).changed()
                    && channels.parse::<u16>().is_ok()
                {
                    config.channels = channels.parse().expect("Should never happen");
                }
            });
    }
}
