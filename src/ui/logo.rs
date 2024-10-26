use egui::{ColorImage, IconData};
use egui_extras::RetainedImage;
use std::sync::OnceLock;

static LOGO: &[u8; 38897] = include_bytes!("../../assets/logo.png");

pub fn icon() -> IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(LOGO)
            .expect("Failed to parse logo.png")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

pub fn logo_image() -> &'static RetainedImage {
    static LOGO_IMAGE: OnceLock<RetainedImage> = OnceLock::new();
    LOGO_IMAGE.get_or_init(|| {
        let image = image::load_from_memory(LOGO)
            .expect("Failed to parse logo.png")
            .into_rgba8();
        let dimensions = image.dimensions();
        let image = ColorImage::from_rgba_unmultiplied(
            [dimensions.0 as usize, dimensions.1 as usize],
            &image.into_raw(),
        );
        RetainedImage::from_color_image("logo", image)
    })
}
