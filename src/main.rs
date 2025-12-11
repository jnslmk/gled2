#![windows_subsystem = "windows"]

pub mod app;
pub mod input;
pub mod midi;
pub mod pipeline;
pub mod storage;
pub mod svg;
pub mod ui;

use app::{App, persistant_state::PersistantState};
use eframe::egui_wgpu::{RenderState, WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
use egui::{Color32, ThemePreference};
use egui_extras::install_image_loaders;
use input::Input;
use once_cell::sync::Lazy;
use pipeline::{constants::OUTPUT_BUFFER_SIZE, renderer_callback::RendererCallback};
use std::sync::{Arc, OnceLock};
use egui_phosphor_icons::add_fonts;
use epaint::FontFamily;
use epaint::text::{FontData, FontDefinitions};
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

#[cfg(target_os = "macos")]
const PRESENT_MODE: PresentMode = PresentMode::Immediate;

#[cfg(not(target_os = "macos"))]
const PRESENT_MODE: PresentMode = PresentMode::Mailbox;

fn main() {
    env_logger::init();
    let ui_action_receiver = UiAction::init_queue();
    RendererCallback::init();
    storage::start_thread();
    ui::temperature::start_thread();
    midi::start_thread();

    #[cfg(not(debug_assertions))]
    ui::update_check::Update::start_thread();

    let mut wgpu_options = WgpuConfiguration::default();
    wgpu_options.present_mode = PRESENT_MODE;
    wgpu_options.wgpu_setup = match wgpu_options.wgpu_setup {
        WgpuSetup::CreateNew(create_new) => WgpuSetup::CreateNew(WgpuSetupCreateNew {
            power_preference: if PersistantState::prefer_discrete_gpu() {
                PowerPreference::HighPerformance
            } else {
                PowerPreference::LowPower
            },
            ..create_new
        }),
        existing => existing,
    };

    let options = eframe::NativeOptions {
        viewport: default_viewport_builder()
            .with_inner_size([1300.0, 1024.0])
            .with_drag_and_drop(true)
            .with_min_inner_size([300.0, 200.0]),
        renderer: eframe::Renderer::Wgpu,
        vsync: false,
        wgpu_options,
        ..Default::default()
    };
    eframe::run_native(
        "gled",
        options,
        Box::new(|cc| {
            let mut fonts = FontDefinitions::default();
            add_fonts(&mut fonts);

            // Register the font by name
            fonts.font_data.insert(
                "Oxanium_Regular".to_owned(),
                Arc::from(FontData::from_static(include_bytes!("../assets/Oxanium-Regular.ttf"))),
            );
            fonts.font_data.insert(
                "Oxanium_Semi_Bold".to_owned(),
                Arc::from(FontData::from_static(include_bytes!("../assets/Oxanium-SemiBold.ttf"))),
            );

            fonts
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .insert(0, "Oxanium_Regular".to_owned());

            fonts.families.insert(FontFamily::Name("Bold".into()), vec!["Oxanium_Semi_Bold".into()]);

            cc.egui_ctx.set_fonts(fonts);

            cc.egui_ctx
                .options_mut(|options| options.theme_preference = ThemePreference::Dark);
            cc.egui_ctx.style_mut(|style| {
                style.always_scroll_the_only_direction = true;
                style.visuals.panel_fill = Color32::from_gray(5);
            });
            install_image_loaders(&cc.egui_ctx);
            Input::init(&cc.egui_ctx);

            WGPU_RENDER_STATE
                .set(
                    cc.wgpu_render_state
                        .clone()
                        .expect("wgpu render state is not available"),
                )
                .map_err(|_err| ())
                .expect("Could not set wgpu render state");
            Ok(Box::new(
                App::new(ui_action_receiver).expect("Could not create new App"),
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
