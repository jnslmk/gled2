use crate::{
    animation::Animation,
    extract::{Extract, Lamp, Positions},
};
use eframe::egui_wgpu::wgpu;
use egui::TextureId;
use std::time::Instant;

pub fn init_shader<'a>(cc: &'a eframe::CreationContext<'a>) -> TextureId {
    let wgpu_render_state = cc
        .wgpu_render_state
        .as_ref()
        .expect("Could not get wgpu render state");
    let device = &wgpu_render_state.device;
    let target_format = wgpu_render_state.target_format;

    let animation = Animation::init(device, target_format);

    let texture_id = wgpu_render_state.renderer.write().register_native_texture(
        device,
        animation.view(),
        wgpu::FilterMode::Nearest,
    );

    let extract = Extract::init(device, animation.texture());

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

pub fn render(frame: &eframe::Frame, start: Instant) {
    let wgpu_render_state = frame
        .wgpu_render_state()
        .expect("Could not get wgpu render state");

    let device = &wgpu_render_state.device;
    let queue = &wgpu_render_state.queue;

    let renderer = wgpu_render_state.renderer.read();

    let animation: &Animation = renderer.paint_callback_resources.get().unwrap();
    animation.prepare(device, queue, start);
    animation.render(device, queue);

    let extract: &Extract = renderer.paint_callback_resources.get().unwrap();
    let mut position = Positions::default();
    position.universes[0].lamps[0] = Lamp::Position { x: 100, y: 42 };
    position.universes[0].lamps[1] = Lamp::Position { x: 100, y: 42 };
    position.universes[0].lamps[2] = Lamp::Position { x: 100, y: 42 };
    position.universes[0].lamps[3] = Lamp::Position { x: 100, y: 42 };
    extract.prepare(queue, &position);
    let artnet_data = extract.run_and_poll(device, queue);
    println!("{:02x?}", &artnet_data[..20]);
}
