/*
Render Shader A -> Texture A
Render Shader B -> Texture B
Compute Shader: Texture A + Mapping A + Texture B + Mapping B -> Artnet Output
Mapping: X+Y pro Artnet Output
*/

mod extract_artnet;
mod shader_widget;
mod animation;
mod texture_to_artnet;
mod scene;
mod mix_artnet;
mod pipeline;

use eframe::egui_wgpu::WgpuConfiguration;
use egui::TextureId;
use shader_widget::{init_shader};

fn main() {
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some([1280.0, 1024.0].into()),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: WgpuConfiguration {
            //present_mode: eframe::wgpu::PresentMode::Immediate,
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "moirë",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc).unwrap())),
    ).expect("Could not run native");
}

struct MyApp {
    texture_id: TextureId,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        shader_widget::render(frame);

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
                    ui.image(self.texture_id, egui::Vec2::splat(800.0));   
                    ui.horizontal(|ui| {
                        for _ in 0..10 {
                            ui.image(self.texture_id, egui::Vec2::splat(80.0));       
                        }             
                    });
                                
                });
        });
        ctx.request_repaint();
    }
}

impl MyApp {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        let texture_id = init_shader(cc);
        Some(Self { texture_id })
    }
}
