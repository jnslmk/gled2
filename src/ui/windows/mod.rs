mod about;
mod artnet_input;
mod output;

#[derive(Default)]
pub struct Windows {
    pub artnet_input: artnet_input::ArtnetInputWindow,
    pub output: output::OutputWindow,
    pub about: about::AboutWindow,
}

impl Windows {
    pub fn update(&mut self, ctx: &egui::Context) {
        self.artnet_input.update(ctx);
        self.output.update(ctx);
        self.about.update(ctx);
    }
}
