#![allow(dead_code)]

mod animation;
mod app;
mod artnet_sender;
mod extract_artnet;
mod logging;
mod logo;
mod mix_artnet;
mod opts;
mod pipeline;
mod scene;
mod shader_widget;
mod svg;
mod texture_to_artnet;

use app::App;
use eframe::egui_wgpu::WgpuConfiguration;
use egui::Vec2;

fn main() {
    logging::init();

    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some([1280.0, 1024.0].into()),
        renderer: eframe::Renderer::Wgpu,
        icon_data: Some(logo::icon()),
        vsync: false,
        wgpu_options: WgpuConfiguration {
            present_mode: eframe::wgpu::PresentMode::Immediate,
            ..Default::default()
        },
        follow_system_theme: false,
        min_window_size: Some(Vec2::new(800.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "gled",
        options,
        Box::new(|cc| Box::new(App::new(cc).expect("Could not create new App"))),
    )
    .expect("Could not run native");
}
