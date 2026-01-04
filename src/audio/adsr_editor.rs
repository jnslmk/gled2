use crate::audio::adsr::{Adsr, AdsrParams, LowPass};
use crate::audio::state::get_fft_data;
use crate::ui::window_common::{default_viewport_builder, gled_window_frame};
use egui::{Color32, Context, Frame, Id, Ui, ViewportId};
use egui_knob::{Knob, KnobStyle, LabelPosition};
use emath::{Pos2, Rect, Vec2, pos2, vec2};
use epaint::{PathStroke, Stroke, StrokeKind};

pub struct ADSREditor {
    adsr: Adsr,
    low_pass: LowPass,
    open: bool,
}

impl Default for ADSREditor {
    fn default() -> Self {
        Self {
            adsr: Adsr::new(AdsrParams::default(),100.),
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
                        ui.horizontal(|ui| {
                            Frame::new().inner_margin(5.).show(ui, |ui| {
                                self.draw_controls(ui);
                            });
                        });
                    });
                });
            },
        );
    }
    fn draw_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let amp = (self.low_pass.tick(&get_fft_data()) * 10. + 1.)
                .log2()
                .clamp(0.0, 1.0);
            self.draw_meter(ui, amp);

            Frame::new().inner_margin(5.).show(ui, |ui| {
                ui.vertical(|ui| {
                    // Sensitivity knob
                    ui.add(
                        knob_default(Knob::new(
                            &mut self.low_pass.trigger_happiness,
                            1.0,
                            100.0,
                            KnobStyle::Wiper,
                        ))
                        .with_size(50.0)
                        .with_label("Sensitivity", LabelPosition::Bottom),
                    );

                    // Threshold knob
                    ui.add(
                        knob_default(Knob::new(
                            &mut self.adsr.params.gate_threshold,
                            0.0,
                            1.0,
                            KnobStyle::Wiper,
                        ))
                        .with_size(30.0)
                        .with_label("Threshold", LabelPosition::Bottom),
                    );
                });
            });
            ui.separator();
            // ADSR knobs
            Frame::new().inner_margin(5.).show(ui, |ui| {
                ui.add(
                    knob_default(Knob::new(
                        &mut self.adsr.params.attack_duration,
                        0.0,
                        2.0,
                        KnobStyle::Wiper,
                    ))
                    .with_size(50.0)
                    .with_label("Attack", LabelPosition::Bottom),
                );

                ui.add(
                    knob_default(Knob::new(
                        &mut self.adsr.params.decay_duration,
                        0.0,
                        10.0,
                        KnobStyle::Wiper,
                    ))
                    .with_size(50.0)
                    .with_label("Decay", LabelPosition::Bottom),
                );
                ui.add(
                    knob_default(Knob::new(
                        &mut self.adsr.params.sustain_level,
                        0.0,
                        1.0,
                        KnobStyle::Wiper,
                    ))
                    .with_size(50.0)
                    .with_label("Sustain", LabelPosition::Bottom),
                );
                ui.add(
                    knob_default(Knob::new(
                        &mut self.adsr.params.release_duration,
                        0.0,
                        10.0,
                        KnobStyle::Wiper,
                    ))
                    .with_size(50.0)
                    .with_label("Release", LabelPosition::Bottom),
                );
            });
            ui.separator();
            let control_amp = self.adsr.tick(amp);
            self.draw_meter(ui, control_amp);
        });
    }

    fn draw_meter(&mut self, ui: &mut Ui, amp: f32) {
        let meter_height = 200.0;
        let (_, rect) = ui.allocate_space(Vec2::new(40.0, meter_height));
        ui.painter().rect_stroke(
            rect,
            0.,
            Stroke::new(2., Color32::WHITE),
            StrokeKind::Outside,
        );

        let mut value_rect = rect.clone();
        value_rect.min.y += meter_height * (1.0 - amp);
        ui.painter()
            .rect_filled(value_rect, 0., Color32::LIGHT_GREEN);

        let threshold_line_y = rect.min.y + meter_height * (1.0 - self.adsr.params.gate_threshold);
        ui.painter().line(
            vec![
                pos2(rect.min.x, threshold_line_y),
                pos2(rect.max.x, threshold_line_y),
            ],
            Stroke::new(2., Color32::BLUE),
        );
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

                let mut adsr = Adsr::new(self.adsr.params.clone(), 100.,);

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let t = i as f32 / (n as f32);
                        let input = if 0.2 < t && t < (0.4 + self.adsr.params.decay_duration * 0.4 + self.adsr.params.attack_duration * 0.4 ) { 1.0 } else { 0.0 };
                        let y = -0.8 * adsr.tick(input) + 1.;
                        to_screen * pos2(t as f32, y)
                    })
                    .collect();

                let thickness = 1.0;
                let shape = epaint::Shape::line(points, PathStroke::new(thickness, Color32::WHITE));

                child_ui.painter().add(shape);
            });
    }
}

fn knob_default(knob: Knob) -> Knob {
    knob.with_font_size(12.0)
        .with_colors(egui::Color32::GRAY, Color32::WHITE, Color32::WHITE)
        .with_stroke_width(3.0)
}
