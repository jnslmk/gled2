use crate::audio::adsr::{Adsr, AdsrParams, LowPass};
use crate::audio::state::get_fft_data;
use crate::ui::window_common::{default_viewport_builder, gled_window_frame};
use egui::{Color32, Context, Frame, Id, Ui, ViewportId};
use emath::{pos2, vec2, Pos2, Rect, Vec2};
use epaint::{PathStroke, Stroke, StrokeKind};

pub struct ADSREditor {
    adsr_params: AdsrParams,
    low_pass: LowPass,
    open: bool,
}

impl Default for ADSREditor {
    fn default() -> Self {
        Self {
            adsr_params: AdsrParams::default(),
            low_pass: LowPass::new(400., 200., 100.),
            open: true,
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
                        ui.debug_paint_cursor();
                        self.draw_meter(ui);
                    });
                });
            },
        );
    }
    fn draw_meter(&mut self, ui: &mut Ui) {
        let (_, mut rect) = ui.allocate_space(Vec2::new(40.0, 200.0));
        let amp = (self.low_pass.tick(&get_fft_data()) + 1.).log2() * 10.;
        ui.painter().rect_stroke(rect, 0., Stroke::new(2., Color32::WHITE), StrokeKind::Outside);

        rect.min.y += 200.0 * (1.0 - amp);
        ui.painter().rect_filled(rect, 0., Color32::LIGHT_GREEN);
    }
    fn draw_curve(&self, ui: &mut Ui) {
        let n = 300;

        Frame::new()
            .outer_margin(5.0)
            .fill(Color32::from_gray(60))
            .show(ui, |child_ui| {
                let desired_size = child_ui.available_width() * vec2(1.0, 0.35);

                let (_id, rect) = child_ui.allocate_space(desired_size);
                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0),
                    rect,
                );

                let mut adsr = Adsr::new(100., self.adsr_params.clone());

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let t = i as f64 / (n as f64);
                        let input = if 0.2 < t && t < 0.4 { 1.0 } else { 0.0 };
                        let y = -0.8 * adsr.tick(input) + 1.;
                        to_screen * pos2(t as f32, y as f32)
                    })
                    .collect();

                let thickness = 1.0;
                let shape = epaint::Shape::line(points, PathStroke::new(thickness, Color32::WHITE));

                child_ui.painter().add(shape);
            });
    }
}
