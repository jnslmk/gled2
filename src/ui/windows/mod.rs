use artnet_input::ArtnetInputWindow;

mod artnet_input;

#[derive(Default)]
pub struct Windows {
    artnet_input_window: ArtnetInputWindow,
}

impl Windows {
    pub fn update(&mut self, ctx: &egui::Context) {
        self.artnet_input_window.update(ctx);
    }
}
