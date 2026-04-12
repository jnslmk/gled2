#![windows_subsystem = "windows"]
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod app;
pub mod audio;
pub mod input;
pub mod midi;
pub mod network_stats;
pub mod pipeline;
pub mod storage;
pub mod svg;
pub mod ui;

use crate::audio::AudioPool;
use crate::input::artnet;
use crate::pipeline::{extract_output, output_sender};
use app::{App, persistant_state::PersistantState};
use clap::Parser;
use eframe::egui_wgpu::{RenderState, WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
use egui::{Color32, ThemePreference};
use egui_extras::install_image_loaders;
use egui_phosphor_icons::add_fonts;
use epaint::FontFamily;
use epaint::text::{FontData, FontDefinitions, FontTweak};
use input::Input;
use once_cell::sync::Lazy;
use pipeline::{constants::OUTPUT_BUFFER_SIZE, renderer_callback::RendererCallback};
use std::sync::{Arc, OnceLock};
use ui::{action::UiAction, window_common::default_viewport_builder};
use wgpu::{Buffer, BufferDescriptor, BufferUsages, PowerPreference, PresentMode};

pub static WGPU_RENDER_STATE: OnceLock<RenderState> = OnceLock::new();
pub static OUTPUT_BUFFER: Lazy<Buffer> = Lazy::new(|| {
    wgpu_render_state().device.create_buffer(&BufferDescriptor {
        size: OUTPUT_BUFFER_SIZE,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        label: Some("Output buffer"),
        mapped_at_creation: false,
    })
});

#[cfg(feature = "profiling")]
static WGPU_PROFILER: Lazy<egui::mutex::Mutex<wgpu_profiler::GpuProfiler>> = Lazy::new(|| {
    egui::mutex::Mutex::new(
        wgpu_profiler::GpuProfiler::new(
            &wgpu_render_state().device,
            wgpu_profiler::GpuProfilerSettings::default(),
        )
        .expect("Could not create WGPU profiler"),
    )
});
#[cfg(feature = "profiling")]
pub static PUFFIN_GPU_PROFILER: Lazy<egui::mutex::Mutex<puffin::GlobalProfiler>> =
    Lazy::new(|| egui::mutex::Mutex::new(puffin::GlobalProfiler::default()));

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Show OpenFont License
    #[arg(short, long)]
    show_open_font_license: bool,
}

fn main() {
    let args = Args::parse();
    if args.show_open_font_license {
        println!(
            "{}",
            std::str::from_utf8(include_bytes!("../assets/OFL.txt"))
                .expect("Could not read OpenFont License")
        );
        return;
    }

    #[cfg(feature = "profiling")]
    let _puffin_servers = start_profile_servers();
    env_logger::init();
    let ui_action_receiver = UiAction::init_queue();
    RendererCallback::init();
    storage::start_thread();
    ui::temperature::start_thread();
    midi::start_thread();
    let network_stats_receiver = network_stats::start_thread();
    let (extract_output, output_receiver) = extract_output::ExtractOutput::new();
    let output_package_sender =
        output_sender::start(output_receiver).expect("Could not start output sender");
    let (artnet_bridge_receiver, artnet_control_receiver) =
        artnet::start_thread(output_package_sender);

    #[cfg(not(debug_assertions))]
    ui::update_check::Update::start_thread();

    let mut wgpu_options = WgpuConfiguration::default();
    wgpu_options.present_mode = PresentMode::AutoNoVsync; // We do not care about vsync as we have our own framerate limiter
    wgpu_options.wgpu_setup = match wgpu_options.wgpu_setup {
        WgpuSetup::CreateNew(create_new) => WgpuSetup::CreateNew(WgpuSetupCreateNew {
            power_preference: if PersistantState::default().prefer_discrete_gpu() {
                PowerPreference::HighPerformance
            } else {
                PowerPreference::LowPower
            },
            #[cfg(feature = "profiling")]
            device_descriptor: std::sync::Arc::new(|adapter| wgpu::DeviceDescriptor {
                required_features: adapter.features()
                    & wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES,
                ..Default::default()
            }),
            ..create_new
        }),
        _ => unreachable!(),
    };

    let options = eframe::NativeOptions {
        viewport: default_viewport_builder()
            .with_inner_size([1300.0, 1024.0])
            .with_drag_and_drop(true)
            .with_min_inner_size([300.0, 200.0]),
        wgpu_options,
        dithering: false,
        ..Default::default()
    };
    eframe::run_native(
        "gled",
        options,
        Box::new(|cc| {
            let mut fonts = FontDefinitions::default();
            add_fonts(&mut fonts);

            let oxanium_tweak = FontTweak {
                scale: 1.0,
                y_offset_factor: 0.15,
                y_offset: 0.0,
                hinting_override: None,
                coords: Default::default(),
            };
            // Register the font by name
            fonts.font_data.insert(
                "Oxanium_Regular".to_owned(),
                Arc::from(
                    FontData::from_static(include_bytes!("../assets/Oxanium-Regular.ttf"))
                        .tweak(oxanium_tweak.clone()),
                ),
            );
            fonts.font_data.insert(
                "Oxanium_Semi_Bold".to_owned(),
                Arc::from(
                    FontData::from_static(include_bytes!("../assets/Oxanium-SemiBold.ttf"))
                        .tweak(oxanium_tweak),
                ),
            );

            fonts
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .insert(0, "Oxanium_Regular".to_owned());

            fonts.families.insert(
                FontFamily::Name("Bold".into()),
                vec!["Oxanium_Semi_Bold".into()],
            );

            cc.egui_ctx.set_fonts(fonts);

            cc.egui_ctx
                .options_mut(|options| options.theme_preference = ThemePreference::Dark);
            cc.egui_ctx.global_style_mut(|style| {
                style.always_scroll_the_only_direction = true;
                style.visuals.panel_fill = Color32::from_gray(5);
            });
            install_image_loaders(&cc.egui_ctx);
            Input::init(&cc.egui_ctx, artnet_bridge_receiver);

            WGPU_RENDER_STATE
                .set(
                    cc.wgpu_render_state
                        .clone()
                        .expect("wgpu render state is not available"),
                )
                .map_err(|_err| ())
                .expect("Could not set wgpu render state");
            let audio_pool = AudioPool::default();
            Ok(Box::new(
                App::new(
                    ui_action_receiver,
                    network_stats_receiver,
                    extract_output,
                    artnet_control_receiver,
                    audio_pool,
                )
                .expect("Could not create new App"),
            ))
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

#[cfg(feature = "profiling")]
struct PuffinViewerChildGuard(std::process::Child);
#[cfg(feature = "profiling")]
impl Drop for PuffinViewerChildGuard {
    fn drop(&mut self) {
        match self.0.kill() {
            Err(e) => println!("Could not kill puffin viewer process: {}", e),
            Ok(_) => println!("Successfully killed puffin viewer process"),
        }
    }
}

#[cfg(feature = "profiling")]
fn start_profile_servers() -> (
    puffin_http::Server,
    puffin_http::Server,
    PuffinViewerChildGuard,
    PuffinViewerChildGuard,
) {
    puffin::set_scopes_on(true);
    let cpu_server = puffin_http::Server::new(&format!("0.0.0.0:{}", puffin_http::DEFAULT_PORT))
        .expect("Could not start puffin server for cpu");
    let gpu_server = puffin_http::Server::new_custom(
        &format!("0.0.0.0:{}", puffin_http::DEFAULT_PORT + 1),
        |sink| PUFFIN_GPU_PROFILER.lock().add_sink(sink),
        |id| _ = PUFFIN_GPU_PROFILER.lock().remove_sink(id),
    )
    .expect("Could not start puffin server for gpu");
    let cpu_profiler =
        PuffinViewerChildGuard(std::process::Command::new("puffin_viewer").spawn().expect(
            "Could not run puffin_viewer, maybe install it with: \"cargo install puffin_viewer\"",
        ));
    let gpu_profiler = PuffinViewerChildGuard(std::process::Command::new("puffin_viewer").arg("--url").arg(format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT + 1)).spawn().expect(
            "Could not run puffin_viewer, maybe install it with: \"cargo install puffin_viewer\"",
        ));
    (cpu_server, gpu_server, cpu_profiler, gpu_profiler)
}
