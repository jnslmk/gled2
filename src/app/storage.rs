use crate::{
    storage::{Loading, STORAGE_DIR},
    ui::{action::UiAction, logo::logo_image},
};
use egui::{
    Color32, Context, Image, Label, Margin, Pos2, Rect, RichText, Spinner, Stroke, Ui, Vec2,
};

fn draw_background_logo(ui: &mut Ui) {
    Image::new(logo_image()).paint_at(ui, {
        let rect = ui.available_rect_before_wrap();
        let size = rect.size();
        if size.x > size.y {
            Rect::from_center_size(rect.center(), Vec2::splat(size.y - 10.0))
        } else {
            Rect::from_center_size(rect.center(), Vec2::splat(size.x - 10.0))
        }
    });
}

pub fn show_storage_loading(ctx: &Context, loading: Loading) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let rect = ui.available_rect_before_wrap();
        let mut spinner_size = 300.0;
        let mut x = rect.center().x;
        let mut y = rect.center().y;

        if y < spinner_size {
            spinner_size = y - 20.0;
        }
        if x < spinner_size {
            spinner_size = x - 20.0;
        }
        x -= spinner_size / 2.0;
        y -= spinner_size / 2.0;
        let center_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(spinner_size));

        draw_background_logo(ui);
        ui.put(center_rect, Spinner::new().size(spinner_size));
        ui.put(
            center_rect,
            Label::new(RichText::new(format!("{loading}")).heading()),
        );
    });
}

pub fn show_storage_error(ctx: &Context, error: String) {
    egui::CentralPanel::default().show(ctx, |ui| {
        draw_background_logo(ui);

        ui.vertical_centered(|ui| {
            ui.add_space(0f32.max((ui.available_height() - 200.0) / 2.0));
            ui.heading("Storage error!");
            egui::Frame::NONE
                .inner_margin(Margin::from(3.0))
                .fill(Color32::DARK_RED)
                .stroke(Stroke::new(1.0, Color32::RED))
                .show(ui, |ui| {
                    ui.add(Label::new(RichText::new(error).color(Color32::WHITE)));
                });
            ui.label(format!("Storage folder: {}", STORAGE_DIR.display()));
            ui.add_space(20.0);
            if ui.button("Open git config").clicked() {
                UiAction::OpenGitConfigWindow.enqueue();
            }
        });
    });
}
