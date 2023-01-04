mod shader_widget;

use std::time::Instant;

use eframe::egui_wgpu::WgpuConfiguration;
use egui::TextureId;
use shader_widget::{init_shader, render_shader_widget};

fn main() {
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some([1280.0, 1024.0].into()),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: WgpuConfiguration {
            present_mode: eframe::wgpu::PresentMode::Immediate,
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "moirë",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc).unwrap())),
    );
}

struct MyApp {
    start: Instant,
    frames: usize,
    texture_id: TextureId,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello Ferris");
            ui.label(format!("fps: {}", self.frames as f32 / self.start.elapsed().as_secs_f32()));

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
                    render_shader_widget(self.start, ui, &self.texture_id);
                    
                    ui.label("Drag to rotate!");
                });
        });
        ctx.request_repaint();
        self.frames += 1;
    }
}

impl MyApp {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        let texture_id = init_shader(cc);
        Some(Self { start: Instant::now(), frames: 0, texture_id })
    }
}
