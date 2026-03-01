use super::{App, svg::Svg};

impl App {
    pub fn svg(&self) -> Option<&Svg> {
        self.project
            .as_ref()
            .and_then(|project| project.svg.as_ref())
    }

    pub fn svg_texture(&mut self, context: &egui::Context) -> Option<egui::TextureHandle> {
        self.project
            .as_mut()?
            .svg
            .as_mut()?
            .image(context, &mut self.extract_output)
    }
}
