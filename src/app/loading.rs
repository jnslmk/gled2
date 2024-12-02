use crate::{storage::Loading, ui::logo::logo_image};
use egui::{
    load::SizedTexture, Color32, Context, Image, Label, Pos2, Rect, RichText, Spinner, Vec2,
};

pub fn show_loading(ctx: &Context, loading: Loading) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let rect = ui.available_rect_before_wrap();
        let mut spinner_size = 300.0;
        let mut x = (rect.max.x - rect.min.x) / 2.0;
        let mut y = (rect.max.y - rect.min.y) / 2.0;

        if y < spinner_size {
            spinner_size = y - 20.0;
        }
        if x < spinner_size {
            spinner_size = x - 20.0;
        }
        x -= spinner_size / 2.0;
        y -= spinner_size / 2.0;
        let center_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(spinner_size));

        ui.put(
            rect,
            Image::new(SizedTexture::new(
                logo_image().texture_id(ctx),
                Vec2::splat(rect.size().min_elem() - 20.0),
            ))
            .tint(Color32::from_white_alpha(1))
            .maintain_aspect_ratio(true),
        );
        ui.put(center_rect, Spinner::new().size(spinner_size));
        ui.put(
            center_rect,
            Label::new(RichText::new(format!("Loading {loading}..")).heading()),
        );
    });
}
