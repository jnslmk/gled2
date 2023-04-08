use super::Svg;
use crate::animation::TEXTURE_SIZE;
use anyhow::Result;
use egui::ColorImage;
use egui_extras::RetainedImage;

impl Svg {
    pub fn render(&self) -> Result<RetainedImage> {
        let mut pixmap = tiny_skia::Pixmap::new(TEXTURE_SIZE, TEXTURE_SIZE).unwrap();
        resvg::render(
            &self.tree,
            resvg::FitTo::Size(TEXTURE_SIZE, TEXTURE_SIZE),
            tiny_skia::Transform::default(),
            pixmap.as_mut(),
        )
        .ok_or_else(|| anyhow::format_err!("Could not render svg"))?;

        let image = ColorImage::from_rgba_unmultiplied([TEXTURE_SIZE as usize; 2], pixmap.data());
        let retained_image = RetainedImage::from_color_image("svg image", image);

        Ok(retained_image)
    }
}
