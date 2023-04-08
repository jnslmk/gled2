#![allow(dead_code)]

mod animation;
mod app;
mod artnet_sender;
mod extract_artnet;
mod icon;
mod logging;
mod mix_artnet;
mod opts;
mod pipeline;
mod scene;
mod shader_widget;
mod svg;
mod texture_to_artnet;

use app::App;
use eframe::egui_wgpu::WgpuConfiguration;

fn main() {
    logging::init();
    artnet_sender::set_artnet_host("127.0.0.1".to_string());

    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some([1280.0, 1024.0].into()),
        renderer: eframe::Renderer::Wgpu,
        icon_data: Some(icon::icon()),
        vsync: false,
        wgpu_options: WgpuConfiguration {
            present_mode: eframe::wgpu::PresentMode::Immediate,
            ..Default::default()
        },
        follow_system_theme: false,
        ..Default::default()
    };
    eframe::run_native(
        "gled",
        options,
        Box::new(|cc| Box::new(App::new(cc).expect("Could not create new App"))),
    )
    .expect("Could not run native");
}
