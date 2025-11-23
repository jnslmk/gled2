use super::ParsedSvg;
use crate::pipeline::constants::PREVIEW_TEXTURE_SIZE;
use anyhow::{Context, Result};
use egui::ColorImage;

impl ParsedSvg {
    pub fn render(&self) -> Result<ColorImage> {
        let mut pixmap =
            tiny_skia::Pixmap::new(PREVIEW_TEXTURE_SIZE as u32, PREVIEW_TEXTURE_SIZE as u32)
                .context("Could not create pixmap for svg")?;
        let size = self.tree.size();

        resvg::render(
            &self.tree,
            tiny_skia::Transform::from_scale(
                f32::from(PREVIEW_TEXTURE_SIZE) / size.width(),
                f32::from(PREVIEW_TEXTURE_SIZE) / size.height(),
            ),
            &mut pixmap.as_mut(),
        );
        let image =
            ColorImage::from_rgba_unmultiplied([PREVIEW_TEXTURE_SIZE as usize; 2], pixmap.data());

        Ok(image)
    }
}
