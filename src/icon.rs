const LOGO: &[u8; 38897] = include_bytes!("logo.png");

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
