mod shader_widget;

use shader_widget::{init_shader, render_shader_widget};

fn main() {
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some([1280.0, 1024.0].into()),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "moirë",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc).unwrap())),
    );
}

#[derive(Default)]
struct MyApp {
    angle: f32,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello Ferris");
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("The triangle is being painted using ");
                        ui.hyperlink_to("WGPU", "https://wgpu.rs");
                        ui.label(" (Portable Rust graphics API awesomeness)");
                    });
                    ui.label("It's not a very impressive demo, but it shows you can embed 3D inside of egui.");
                    render_shader_widget(&mut self.angle, ui);
                    
                    ui.label("Drag to rotate!");
                });
        });
    }
}

impl MyApp {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        init_shader(cc)?;
        Some(Self { angle: 0.0 })
    }
}
