use super::{App, Svg};

impl App {
    pub fn set_svg(&mut self, svg: Option<Svg>) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        project.svg = svg;
    }

    pub fn svg(&self) -> Option<&Svg> {
        self.project
            .as_ref()
            .and_then(|project| project.svg.as_ref())
    }

    pub fn svg_mut(&mut self) -> Option<&mut Svg> {
        self.project
            .as_mut()
            .and_then(|project| project.svg.as_mut())
    }
}
