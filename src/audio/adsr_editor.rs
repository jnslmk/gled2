use crate::audio::adsr::{Adsr, AdsrParams};
use crate::ui::window_common::{default_viewport_builder, gled_window_frame};
use egui::{Color32, Context, Id, Ui, UiBuilder, ViewportId};
use emath::{pos2, vec2, Pos2, Rect, Vec2};
use epaint::PathStroke;

pub struct ADSREditor {
    adsr_params: AdsrParams,
    center_bin: usize,
    use_bins: usize,
    open: bool,
}

impl Default for ADSREditor {
    fn default() -> Self {
        Self {
            adsr_params: AdsrParams::default(),
            center_bin: 0,
            use_bins: 100,
            open: false,
        }
    }
}

impl ADSREditor {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        ctx.show_viewport_immediate(
            ViewportId(Id::new("ADSR Editor")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(660.0, 500.0))
                .with_min_inner_size(Vec2::new(660.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "ADSR Editor", |ui| {
                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.draw_curve(ui);
                    });
                });
            },
        );
    }

    fn draw_curve(&self, parent_ui: &mut Ui) {
        let n = 300;

        let builder = UiBuilder::new();
        let mut child_ui = parent_ui.new_child(builder);
        child_ui.take_available_space();

        let desired_size = child_ui.available_width() * vec2(1.0, 0.35);
        let (_id, rect) = child_ui.allocate_space(desired_size);
        let to_screen =
            emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, 0.0 ..=1.0), rect);

        let mut adsr = Adsr::new(100. , self.adsr_params.clone());

        let points: Vec<Pos2> = (0..=n)
            .map(|i| {
                let t = i as f64 / (n as f64);
                let input =  if  0.2 < t && t < 0.4 {1.0} else {0.0};
                let y = -0.8 *adsr.tick(input) + 1.;
                to_screen * pos2(t as f32, y as f32)
            })
            .collect();

        let thickness = 1.0;
        let shape = epaint::Shape::line(
            points,
            PathStroke::new(thickness, Color32::WHITE)
        );

        child_ui.painter().rect_filled(rect, 0.0, Color32::from_gray(100));
        child_ui.painter().add(shape);
    }
}
