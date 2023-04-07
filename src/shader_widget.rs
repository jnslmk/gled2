use crate::{
    animation::{Animation, Color, ColorPalette, Config},
    extract::{Extract, Lamp, Positions},
};
use eframe::egui_wgpu::wgpu;
use egui::TextureId;

pub struct Scene {
    pub animation: Animation,
    pub extract: Extract,
}

pub struct Pipeline {
    scenes: Vec<Scene>,
}

pub fn init_shader<'a>(cc: &'a eframe::CreationContext<'a>) -> TextureId {
    let wgpu_render_state = cc
        .wgpu_render_state
        .as_ref()
        .expect("Could not get wgpu render state");
    let device = &wgpu_render_state.device;

    let palette = ColorPalette {
        colors: vec![
            Color::new(1., 0., 0.7),
            Color::new(0., 0.2, 0.2),
            Color::new(0., 0., 0.),
            Color::new(0., 0.4, 0.5),
            Color::new(0., 1., 0.),
        ],
    };

    let config = Config {
        center: (0.75, 0.25),
        thickness: 0.01,
        count: 12,
    };

    let animation = Animation::init(device, &palette, &config);

    let texture_id = wgpu_render_state.renderer.write().register_native_texture(
        device,
        animation.view(),
        wgpu::FilterMode::Nearest,
    );

    let mut positions = Positions::default();
    positions.universes[0].lamps[0] = Lamp::Position { x: 100, y: 42 };
    positions.universes[0].lamps[1] = Lamp::Position { x: 100, y: 42 };
    positions.universes[0].lamps[2] = Lamp::Position { x: 100, y: 42 };
    positions.universes[0].lamps[3] = Lamp::Position { x: 100, y: 42 };
    let extract = Extract::init(device, animation.texture(), &positions);

    // Because the graphics pipeline must have the same lifetime as the egui render pass,
    // instead of storing the pipeline in our `Custom3D` struct, we insert it into the
    // `paint_callback_resources` type map, which is stored alongside the render pass.
    wgpu_render_state
        .renderer
        .write()
        .paint_callback_resources
        .insert(animation);
    wgpu_render_state
        .renderer
        .write()
        .paint_callback_resources
        .insert(extract);

    texture_id
}

pub fn render(frame: &eframe::Frame) {
    let wgpu_render_state = frame
        .wgpu_render_state()
        .expect("Could not get wgpu render state");

    let device = &wgpu_render_state.device;
    let queue = &wgpu_render_state.queue;

    let renderer = wgpu_render_state.renderer.read();

    let animation: &Animation = renderer.paint_callback_resources.get().unwrap();
    animation.prepare(queue);
    animation.render(device, queue);

    let extract: &Extract = renderer.paint_callback_resources.get().unwrap();

    let artnet_data = extract.run_and_poll(device, queue);
    println!("{:02x?}", &artnet_data[..16]);
}
