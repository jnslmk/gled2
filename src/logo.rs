use egui::ColorImage;
use egui_extras::RetainedImage;

const LOGO: &[u8; 38897] = include_bytes!("../assets/logo.png");

pub fn icon() -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(LOGO)
            .expect("Failed to parse logo.png")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

pub fn logo_image() -> RetainedImage {
    let image = image::load_from_memory(LOGO)
        .expect("Failed to parse logo.png")
        .into_rgba8();
    let dimensions = image.dimensions();
    let image = ColorImage::from_rgba_unmultiplied(
        [dimensions.0 as usize, dimensions.1 as usize],
        &image.into_raw(),
    );
    RetainedImage::from_color_image("logo", image)
}
