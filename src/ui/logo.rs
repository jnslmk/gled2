use egui::{IconData, ImageSource};

static LOGO: &[u8; 38897] = include_bytes!("../../logo/logo.png");

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

pub fn logo_image() -> ImageSource<'static> {
    const IMAGE_SOURCE: ImageSource = ImageSource::Bytes {
        uri: std::borrow::Cow::Borrowed("../../logo/logo.png"),
        bytes: egui::load::Bytes::Static(LOGO),
    };
    IMAGE_SOURCE
}
