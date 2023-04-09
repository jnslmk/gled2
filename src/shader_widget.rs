use crate::{
    animation::{
        Color, ColorPalette, CommonConfig, Direction, Gradient, GradientConfig, GradientType,
        Stripes, StripesConfig,
    },
    artnet_sender::ArtnetSender,
    pipeline::Pipeline,
    scene::Scene,
    svg::{MeasurementPoints, Universes},
};
use eframe::egui_wgpu::{wgpu, RenderState};
use egui::TextureId;

pub fn init(render_state: &RenderState, measurement_points: &MeasurementPoints) -> Vec<TextureId> {
    let device = &render_state.device;
    let mut texture_ids = vec![];
    let mut pipeline = Pipeline::init(device);

    let palette = ColorPalette {
        colors: vec![Color::new(1., 0., 0.5), Color::new(0., 0., 0.)],
    };
    let gradient = Gradient::new(
        device,
        &palette,
        GradientConfig {
            gradient: GradientType::Radial {
                center: (0.25, 0.5),
            },
            ..Default::default()
        },
    );
    texture_ids.push(render_state.renderer.write().register_native_texture(
        device,
        gradient.renderer().view(),
        wgpu::FilterMode::Nearest,
    ));
    let positions = measurement_points.positions("allFull").unwrap_or_default();
    let scene = Scene::new(device, gradient.into(), &positions);
    pipeline.add_scene(scene);

    let palette = ColorPalette {
        colors: vec![Color::new(0., 0., 1.), Color::new(0., 0., 0.)],
    };
    let gradient = Gradient::new(
        device,
        &palette,
        GradientConfig {
            gradient: GradientType::LinearHorizontal,
            common: CommonConfig {
                ..Default::default()
            },
        },
    );
    texture_ids.push(render_state.renderer.write().register_native_texture(
        device,
        gradient.renderer().view(),
        wgpu::FilterMode::Nearest,
    ));
    let positions = measurement_points
        .positions("innerEdge")
        .unwrap_or_default();
    let scene = Scene::new(device, gradient.into(), &positions);
    pipeline.add_scene(scene);

    let palette = ColorPalette {
        colors: vec![Color::new(1., 1., 0.)],
    };
    let stripes = Stripes::new(
        device,
        &palette,
        StripesConfig {
            count: 2,
            common: CommonConfig {
                direction: Direction::Backward,
            },
            ..Default::default()
        },
    );
    texture_ids.push(render_state.renderer.write().register_native_texture(
        device,
        stripes.renderer().view(),
        wgpu::FilterMode::Nearest,
    ));
    let positions = measurement_points
        .positions("innerFull")
        .unwrap_or_default();
    let scene = Scene::new(device, stripes.into(), &positions);
    pipeline.add_scene(scene);

    render_state
        .renderer
        .write()
        .paint_callback_resources
        .insert(pipeline);

    texture_ids
}

pub fn render(
    frame: &eframe::Frame,
    universes: &Universes,
    artnet_sender: &mut ArtnetSender,
    beat_progression: f32,
    beats_per_minute: f32,
    framerate: f32,
    blackout: bool,
) {
    let wgpu_render_state = frame
        .wgpu_render_state()
        .expect("Could not get wgpu render state");

    let device = &wgpu_render_state.device;
    let queue = &wgpu_render_state.queue;

    let mut renderer = wgpu_render_state.renderer.write();

    let pipeline: &mut Pipeline = renderer
        .paint_callback_resources
        .get_mut()
        .expect("Could not find Pipeline");

    let commands = pipeline
        .run_and_poll(
            device,
            queue,
            universes,
            beat_progression,
            beats_per_minute,
            framerate,
            blackout,
        )
        .expect("No scene registered");

    for command in commands {
        artnet_sender
            .send(command)
            .expect("Artnet sender closed its channel");
    }
}
