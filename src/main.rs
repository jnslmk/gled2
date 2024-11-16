#![windows_subsystem = "windows"]
#![allow(deprecated)]

mod animation;
mod app;
mod color_shift;
mod constants;
mod effect;
mod extract_output;
mod group;
mod input;
mod logging;
mod opts;
mod output_clear;
mod output_mix;
mod output_sender;
mod pipeline;
mod preview;
mod preview_indices;
mod project;
mod scene_instance;
mod storage;
mod svg;
mod texture_to_output;
mod transition;
mod ui;

use app::App;
use eframe::egui_wgpu::{RenderState, WgpuConfiguration};
use egui::ViewportBuilder;
use input::Input;
use std::sync::OnceLock;
use ui::logo::icon;
use wgpu::PowerPreference;

pub static WGPU_RENDER_STATE: OnceLock<RenderState> = OnceLock::new();

fn main() {
    logging::init();
    storage::start_thread();

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1300.0, 1024.0])
            .with_drag_and_drop(true)
            .with_min_inner_size([300.0, 200.0])
            .with_icon(icon()),
        renderer: eframe::Renderer::Wgpu,
        vsync: false,
        wgpu_options: WgpuConfiguration {
            present_mode: eframe::wgpu::PresentMode::Mailbox,
            power_preference: PowerPreference::HighPerformance,
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "gled",
        options,
        Box::new(|cc| {
            Input::init(&cc.egui_ctx);

            WGPU_RENDER_STATE
                .set(
                    cc.wgpu_render_state
                        .clone()
                        .expect("wgpu render state is not available"),
                )
                .map_err(|_err| ())
                .expect("Could not set wgpu render state");
            Ok(Box::new(App::new().expect("Could not create new App")))
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
