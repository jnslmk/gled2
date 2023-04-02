use crate::animation::Animation;
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
        &animation.view,
        wgpu::FilterMode::Nearest,
    );

    // Because the graphics pipeline must have the same lifetime as the egui render pass,
    // instead of storing the pipeline in our `Custom3D` struct, we insert it into the
    // `paint_callback_resources` type map, which is stored alongside the render pass.
    wgpu_render_state
        .renderer
        .write()
        .paint_callback_resources
        .insert(animation);

    texture_id
}

pub fn render(frame: &eframe::Frame, start: Instant) {
    let wgpu_render_state = frame
        .wgpu_render_state()
        .expect("Could not get wgpu render state");

    let device = &wgpu_render_state.device;
    let queue = &wgpu_render_state.queue;

    let renderer = wgpu_render_state.renderer.read();
    let resources: &Animation = renderer.paint_callback_resources.get().unwrap();

    resources.prepare(device, queue, start);

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("GIF Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &resources.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }),
                store: true,
            },
        })],
        depth_stencil_attachment: None,
    });

    resources.paint(&mut rpass);

    drop(rpass);

    /*     encoder.copy_buffer_to_buffer(
        &resources.artnet_buffer_gpu,
        0,
        &resources.artnet_buffer_cpu,
        0,
        65536,
    ); */

    queue.submit(std::iter::once(encoder.finish()));

    /*     // Create the map request
    let buffer_slice = resources.artnet_buffer_cpu.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    }); */

    device.poll(wgpu::Maintain::Wait);

    /*  match rx.recv().unwrap() {
        Ok(()) => {
            let padded_data = buffer_slice.get_mapped_range();
            /*let data = padded_data
            .chunks(padded_bytes_per_row as _)
            .flat_map(|chunk| &chunk[..unpadded_bytes_per_row as _])
            .copied()
            .collect::<Vec<_>>();
            */
            //dbg!(&padded_data[..4]);
            drop(padded_data);

            resources.artnet_buffer_cpu.unmap();
        }
        _ => {
            eprintln!("Something went wrong")
        }
    } */
}
