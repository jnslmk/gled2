#![allow(dead_code)]

/*
Render Shader A -> Texture A
Render Shader B -> Texture B
Compute Shader: Texture A + Mapping A + Texture B + Mapping B -> Artnet Output
Mapping: X+Y pro Artnet Output
*/

mod animation;
mod extract_artnet;
mod mix_artnet;
mod pipeline;
mod scene;
mod shader_widget;
mod texture_to_artnet;

use eframe::egui_wgpu::WgpuConfiguration;
use egui::TextureId;
use shader_widget::init_shader;

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
        "gled2",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc).unwrap())),
    )
    .expect("Could not run native");
}

struct MyApp {
    texture_ids: Vec<TextureId>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        shader_widget::render(frame);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for texture_id in self.texture_ids.iter() {
                            ui.image(*texture_id, egui::Vec2::splat(300.0));
                        }
                    });
                });
        });
        ctx.request_repaint();
    }
}

impl MyApp {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        let texture_ids = init_shader(cc);
        Some(Self { texture_ids })
    }
}
