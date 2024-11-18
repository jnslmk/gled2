use crate::{input::ARTNET_INPUT_CONFIG, viewport_builder::default_viewport_builder};
use egui::{Context, Id, TextEdit, Vec2, ViewportId};

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

        ctx.show_viewport_immediate(
            ViewportId(Id::new("artnet inputs window")),
            default_viewport_builder()
                .with_title("Gled: Artnet Inputs")
                .with_inner_size(Vec2::new(160.0, 90.0))
                .with_resizable(false)
                .with_minimize_button(false)
                .with_maximize_button(false),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                egui::CentralPanel::default().show(ctx, |ui| {
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
                    if ui.add(TextEdit::singleline(start)).changed() && start.parse::<u16>().is_ok()
                    {
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
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
