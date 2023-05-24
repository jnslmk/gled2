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
        let tree = resvg::Tree::from_usvg(&self.tree);
        let max_side = tree.size.width().max(tree.size.height());
        let scale = f32::from(PREVIEW_TEXTURE_SIZE) / max_side as f32;

        resvg::Tree::render(
            &tree,
            tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        let image =
            ColorImage::from_rgba_unmultiplied([PREVIEW_TEXTURE_SIZE as usize; 2], pixmap.data());
        let retained_image = RetainedImage::from_color_image("svg image", image);

        Ok(retained_image)
    }
}
