use super::App;
use egui::{load::SizedTexture, Context, Layout, RichText, Vec2};

impl App {
    pub fn about_window(&mut self, ctx: &Context) {
        if self.about_window_open {
            egui::Window::new("About")
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.available_rect().center())
                .open(&mut self.about_window_open)
                .show(ctx, |ui| {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.image(SizedTexture::new(
                            self.logo_image.texture_id(ctx),
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
    }
}
