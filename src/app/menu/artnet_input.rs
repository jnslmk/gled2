use crate::{app::App, artnet_receiver::ARTNET_CONFIG};
use egui::{Context, TextEdit};

impl App {
    pub fn config_artnet_input_window(&mut self, ctx: &Context) {
        if self.artnet_input_window_open {
            egui::Window::new("Config Artnet Inputs")
                .collapsible(false)
                .resizable(true)
                .default_pos(ctx.available_rect().center())
                .open(&mut self.artnet_input_window_open)
                .show(ctx, |ui| {
                    let mut config = ARTNET_CONFIG.lock().expect("ARTNET_CONFIG is poisoned");

                    ui.label("Universe");
                    let universe = self
                        .inputs
                        .entry("artnet_input_universe".to_owned())
                        .or_insert_with(|| config.universe.to_string());
                    if ui.add(TextEdit::singleline(universe)).changed()
                        && universe.parse::<u16>().is_ok()
                    {
                        config.universe = universe.parse().expect("Should never happen");
                    }

                    ui.label("Start channel");
                    let start = self
                        .inputs
                        .entry("artnet_input_start".to_owned())
                        .or_insert_with(|| config.start.to_string());
                    if ui.add(TextEdit::singleline(start)).changed() && start.parse::<u16>().is_ok()
                    {
                        config.start = start.parse().expect("Should never happen");
                    }

                    ui.label("Channels");
                    let channels = self
                        .inputs
                        .entry("artnet_input_channels".to_owned())
                        .or_insert_with(|| config.channels.to_string());
                    if ui.add(TextEdit::singleline(channels)).changed()
                        && channels.parse::<u16>().is_ok()
                    {
                        config.channels = channels.parse().expect("Should never happen");
                    }
                });
        }
    }
}
