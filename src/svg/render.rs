use crate::constants::PREVIEW_TEXTURE_SIZE;

use super::ParsedSvg;
use anyhow::{Context, Result};
use egui::ColorImage;
use egui_extras::RetainedImage;

impl ParsedSvg {
    pub fn render(&self) -> Result<RetainedImage> {
        let mut pixmap =
            tiny_skia::Pixmap::new(PREVIEW_TEXTURE_SIZE as u32, PREVIEW_TEXTURE_SIZE as u32)
                .context("Could not create pixmap for svg")?;
        resvg::render(
            &self.tree,
            resvg::FitTo::Size(PREVIEW_TEXTURE_SIZE as u32, PREVIEW_TEXTURE_SIZE as u32),
            tiny_skia::Transform::default(),
            pixmap.as_mut(),
        )
        .ok_or_else(|| anyhow::format_err!("Could not render svg"))?;

        let image =
            ColorImage::from_rgba_unmultiplied([PREVIEW_TEXTURE_SIZE as usize; 2], pixmap.data());
        let retained_image = RetainedImage::from_color_image("svg image", image);

        Ok(retained_image)
    }
}
