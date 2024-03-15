#![windows_subsystem = "windows"]
#![allow(dead_code)]
#![allow(deprecated)]

mod animation;
mod app;
mod artnet_receiver;
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
mod storage;
mod svg;
mod texture_to_output;
mod transition;

use app::App;
use eframe::egui_wgpu::{RenderState, WgpuConfiguration};
use egui::ViewportBuilder;
use once_cell::sync::OnceCell;
use wgpu::PowerPreference;

pub static WGPU_RENDER_STATE: OnceCell<RenderState> = OnceCell::new();

fn main() {
    logging::init();
    let receiver = artnet_receiver::start_thread();

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1300.0, 1024.0])
            .with_drag_and_drop(true)
            .with_min_inner_size([300.0, 200.0])
            .with_icon(logo::icon()),
        renderer: eframe::Renderer::Wgpu,
        vsync: false,
        wgpu_options: WgpuConfiguration {
            present_mode: eframe::wgpu::PresentMode::Mailbox,
            power_preference: PowerPreference::HighPerformance,
            ..Default::default()
        },
        follow_system_theme: false,
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
            Box::new(App::new(receiver).expect("Could not create new App"))
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
