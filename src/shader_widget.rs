use crate::{
    animation::{
        Color, ColorPalette, CommonConfig, Direction, Gradient, GradientConfig, GradientType,
        Stripes, StripesConfig,
    },
    artnet_sender::ArtnetSender,
    pipeline::Pipeline,
    scene::Scene,
    svg::Universes,
    wgpu_render_state,
};

pub fn init_shaders() {
    let wgpu_render_state = wgpu_render_state();
    let device = &wgpu_render_state.device;
    let mut pipeline = Pipeline::init(device);

    let palette = ColorPalette {
        colors: vec![Color::new(1., 0., 0.5), Color::new(0., 0., 0.)],
    };
    let gradient = Gradient::new(
        device,
        GradientConfig {
            gradient: GradientType::Radial {
                center: (0.25, 0.5),
            },
            ..Default::default()
        },
    );
    let scene = Scene::new(device, gradient.into(), palette, "allFull".to_owned());
    pipeline.add_scene(scene);

    let palette = ColorPalette {
        colors: vec![Color::new(0., 0., 1.), Color::new(0., 0., 0.)],
    };
    let gradient = Gradient::new(
        device,
        GradientConfig {
            gradient: GradientType::LinearHorizontal,
            common: CommonConfig {
                ..Default::default()
            },
        },
    );
    let scene = Scene::new(device, gradient.into(), palette, "innerEdge".to_owned());
    pipeline.add_scene(scene);

    let palette = ColorPalette {
        colors: vec![Color::new(1., 1., 0.)],
    };
    let stripes = Stripes::new(
        device,
        StripesConfig {
            count: 2,
            common: CommonConfig {
                direction: Direction::Backward,
            },
            ..Default::default()
        },
    );
    let scene = Scene::new(device, stripes.into(), palette, "innerFull".to_owned());
    pipeline.add_scene(scene);

    wgpu_render_state
        .renderer
        .write()
        .paint_callback_resources
        .insert(pipeline);
}

pub fn render(
    universes: &Universes,
    artnet_sender: &mut ArtnetSender,
    beat_progression: f32,
    beats_per_minute: f32,
    framerate: f32,
    disable_artnet_extraction: bool,
) {
    let wgpu_render_state = wgpu_render_state();
    let device = &wgpu_render_state.device;
    let queue = &wgpu_render_state.queue;

    crate::get_pipeline!(pipeline);
    let commands = pipeline
        .run_and_poll(
            device,
            queue,
            universes,
            beat_progression,
            beats_per_minute,
            framerate,
            disable_artnet_extraction,
        )
        .expect("No scene registered");

    for command in commands {
        artnet_sender
            .send(command)
            .expect("Artnet sender closed its channel");
    }
}
