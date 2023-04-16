//TODO: Remove

use crate::{
    animation::{
        Color, ColorPalette, CommonConfig, Direction, Gradient, GradientConfig, GradientType,
        Stripes, StripesConfig,
    },
    extract_artnet::ExtractArtnet,
    pipeline::Pipeline,
    scene::{Scene, SceneKind},
};

pub fn init_shaders(extract_artnet: ExtractArtnet) {
    let mut pipeline = Pipeline::default();
    pipeline.set_extract_artnet(extract_artnet);

    for i in 0..10 {
        let palette = ColorPalette {
            colors: vec![Color::new(1., 0., 0.), Color::new(0., 0., 0.)],
        };
        let gradient = Gradient::new(GradientConfig {
            gradient: GradientType::Radial {
                center: (0.25, 0.5),
            },
            ..Default::default()
        });
        let mut scene = Scene::new(gradient.into(), palette, "allFull".to_owned());
        scene.artnet_extraction = i == 0;
        pipeline.add_scene(scene);

        let palette = ColorPalette {
            colors: vec![Color::new(0., 0., 1.), Color::new(0., 0., 0.)],
        };
        let gradient = Gradient::new(GradientConfig {
            gradient: GradientType::LinearHorizontal,
            common: CommonConfig {
                ..Default::default()
            },
        });
        let mut scene = Scene::new(gradient.into(), palette, "innerFull".to_owned());
        scene.artnet_extraction = i == 0;
        pipeline.add_scene(scene);

        let palette = ColorPalette {
            colors: vec![Color::new(1., 1., 0.)],
        };
        let stripes = Stripes::new(StripesConfig {
            count: 2,
            common: CommonConfig {
                direction: Direction::Backward,
            },
            ..Default::default()
        });
        let mut scene = Scene::new(stripes.into(), palette, "innerEdge".to_owned());
        scene.kind = SceneKind::Foreground;
        scene.artnet_extraction = i == 0;
        pipeline.add_scene(scene);
    }
}
