use crate::ui::logo::icon;
use egui::ViewportBuilder;

pub fn default_viewport_builder() -> ViewportBuilder {
    ViewportBuilder::default()
        .with_title("Gled")
        .with_app_id("de.photonenkollektiv.gled")
        .with_icon(icon())
}
