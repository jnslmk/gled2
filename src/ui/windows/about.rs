use crate::ui::{
    logo::logo_image,
    window_common::{default_viewport_builder, gled_window_frame},
};
use egui::{Id, Image, Layout, RichText, Vec2, ViewportId};

#[derive(Default)]
pub struct AboutWindow {
    open: bool,
}

impl AboutWindow {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("about window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(280.0, 250.0))
                .with_minimize_button(false)
                .with_maximize_button(false)
                .with_resizable(false),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "About", |ui| {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.add(Image::new(logo_image()).fit_to_exact_size(Vec2::splat(50.0)));
                        ui.label(RichText::new("gled").text_style(egui::TextStyle::Heading));
                        ui.spacing();
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.spacing();
                        ui.hyperlink_to(
                            "Made with ❤ for Photonenkollektiv",
                            "https://www.photonenkollektiv.de",
                        );
                        ui.spacing();
                        ui.hyperlink_to(
                            "Project website",
                            "https://photonenkollektiv.gitlab.io/gled2/",
                        );
                        ui.spacing();
                        ui.label("Logo artwork created by Geoffrey Guterl.");
                        ui.hyperlink_to(
                            "Logo creation sponsored by FreshX GmbH",
                            "https://www.freshx.de",
                        );
                    });
                });
            },
        );
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }
}
