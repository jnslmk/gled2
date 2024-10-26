use crate::ui::logo::logo_image;
use egui::{load::SizedTexture, Layout, RichText, Vec2};

#[derive(Default)]
pub struct AboutWindow {
    open: bool,
}

impl AboutWindow {
    pub fn update(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        egui::Window::new("About")
            .collapsible(false)
            .resizable(false)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                    ui.image(SizedTexture::new(
                        logo_image().texture_id(ctx),
                        Vec2::splat(500.0),
                    ));
                    ui.label(RichText::new("gled").text_style(egui::TextStyle::Heading));
                    ui.spacing();
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.spacing();
                    ui.label("Made with Rust ❤");
                    ui.spacing();
                    ui.hyperlink_to("Project website", "https://pentagonum.gitlab.io/gled/");
                    ui.spacing();
                    ui.label("Logo artwork created by Geoffrey Guterl.");
                    ui.hyperlink_to(
                        "Logo creation sponsored by FreshX GmbH",
                        "https://www.freshx.de",
                    );
                });
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
