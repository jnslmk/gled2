use crate::{
    animation::{Color, ColorPalette, Gradient, GradientConfig, GradientType},
    pipeline::Pipeline,
    scene::Scene,
    texture_to_artnet::{Lamp, Positions},
};
use eframe::egui_wgpu::wgpu;
use egui::TextureId;

pub fn init_shader<'a>(cc: &'a eframe::CreationContext<'a>) -> Vec<TextureId> {
    let wgpu_render_state = cc
        .wgpu_render_state
        .as_ref()
        .expect("Could not get wgpu render state");
    let device = &wgpu_render_state.device;

    let mut texture_ids = vec![];
    let mut pipeline = Pipeline::init(device);

    let palette = ColorPalette {
        colors: vec![Color::new(1., 0., 0.5), Color::new(0., 0., 0.)],
    };
    let gradient = Gradient::new(
        device,
        &palette,
        GradientConfig {
            gradient: GradientType::Radial,
            ..Default::default()
        },
    );
    texture_ids.push(wgpu_render_state.renderer.write().register_native_texture(
        device,
        gradient.renderer().view(),
        wgpu::FilterMode::Nearest,
    ));
    let mut positions = Positions::default();
    positions.universes[0].lamps[0] = Lamp::Position { x: 500, y: 42 };
    positions.universes[0].lamps[1] = Lamp::Position { x: 500, y: 42 };
    positions.universes[0].lamps[2] = Lamp::Position { x: 500, y: 42 };
    positions.universes[0].lamps[3] = Lamp::Position { x: 500, y: 42 };
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
            opacity: 0.2,
            ..Default::default()
        },
    );
    texture_ids.push(wgpu_render_state.renderer.write().register_native_texture(
        device,
        gradient.renderer().view(),
        wgpu::FilterMode::Nearest,
    ));
    let mut positions = Positions::default();
    positions.universes[0].lamps[0] = Lamp::Position { x: 80, y: 42 };
    positions.universes[0].lamps[1] = Lamp::Position { x: 500, y: 42 };
    positions.universes[0].lamps[2] = Lamp::Position { x: 80, y: 42 };
    let scene = Scene::new(device, gradient.into(), &positions);
    pipeline.add_scene(scene);

    wgpu_render_state
        .renderer
        .write()
        .paint_callback_resources
        .insert(pipeline);

    texture_ids
}

pub fn render(frame: &eframe::Frame) {
    let wgpu_render_state = frame
        .wgpu_render_state()
        .expect("Could not get wgpu render state");

    let device = &wgpu_render_state.device;
    let queue = &wgpu_render_state.queue;

    let mut renderer = wgpu_render_state.renderer.write();

    let pipeline: &mut Pipeline = renderer.paint_callback_resources.get_mut().unwrap();

    let artnet_data = pipeline
        .run_and_poll(device, queue)
        .expect("No scene registered");

    println!("{:02x?}", &artnet_data[..16]);
}
