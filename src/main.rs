#![allow(dead_code)]

mod animation;
mod app;
mod constants;
mod extract_output;
mod hotkey;
mod logging;
mod logo;
mod opts;
mod output_clear;
mod output_mix;
mod output_sender;
mod pipeline;
mod preview;
mod preview_indices;
mod project;
mod scene;
mod svg;
mod texture_to_output;
mod transition;

use app::App;
use eframe::egui_wgpu::{RenderState, WgpuConfiguration};
use egui::Vec2;
use once_cell::sync::OnceCell;

pub static WGPU_RENDER_STATE: OnceCell<RenderState> = OnceCell::new();

fn main() {
    logging::init();

    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some([1300.0, 1024.0].into()),
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
        Box::new(|cc| {
            WGPU_RENDER_STATE
                .set(
                    cc.wgpu_render_state
                        .clone()
                        .expect("wgpu render state is not available"),
                )
                .map_err(|_err| ())
                .expect("Could not set wgpu render state");
            Box::new(App::new().expect("Could not create new App"))
        }),
    )
    .expect("Could not run native");
}

pub fn wgpu_render_state() -> RenderState {
    WGPU_RENDER_STATE
        .get()
        .expect("Could not find wgpu render state")
        .clone()
}
