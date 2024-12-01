use crossbeam_channel::Sender;
use egui::{mutex::Mutex, Color32, Pos2, Stroke};
use epaint::QuadraticBezierShape;
use ndarray::arr1;
use once_cell::sync::Lazy;
use polyfit_residuals::{poly::NewtonPolynomial, try_fit_poly_with_residual, PolyFit};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    thread::spawn,
};

pub type Polynomial = NewtonPolynomial<f64, f64, Vec<f64>, Vec<f64>>;

static POLYNOMIALS: Lazy<Mutex<HashMap<BezierCurve, Option<Polynomial>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static POLYNOMIAL_QUEUE: Lazy<Sender<BezierCurve>> = Lazy::new(|| {
    let (sender, receiver) = crossbeam_channel::unbounded::<BezierCurve>();
    spawn(move || {
        for bezier_curve in receiver {
            POLYNOMIALS
                .lock()
                .insert(bezier_curve, Some(bezier_curve.polynomial()));
        }
    });
    sender
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BezierCurve {
    pub start: Pos2,
    pub bezier: Pos2,
    pub end: Pos2,
}

impl Hash for BezierCurve {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.x.to_be_bytes().hash(state);
        self.start.y.to_be_bytes().hash(state);
        self.bezier.x.to_be_bytes().hash(state);
        self.bezier.y.to_be_bytes().hash(state);
        self.end.x.to_be_bytes().hash(state);
        self.end.y.to_be_bytes().hash(state);
    }
}

impl BezierCurve {
    pub fn new(start: Pos2, bezier: Pos2, end: Pos2) -> Self {
        Self { start, bezier, end }
    }

    pub fn polynomial(&self) -> Polynomial {
        log::debug!("Fitting polynomial to curve: {self:?}");

        let curve = QuadraticBezierShape::from_points_stroke(
            [self.start, self.bezier, self.end],
            false,
            Color32::TRANSPARENT,
            Stroke::default(),
        );

        let (x_values, y_values): (Vec<_>, Vec<_>) = (0..1_000_000)
            .map(|t| t as f32 / 1_000_000.0)
            .map(|t| {
                let pos = curve.sample(t);
                (pos.x as f64, pos.y as f64)
            })
            .unzip();

        let x_values = arr1(&x_values);
        let y_values = arr1(&y_values);

        let mut polynomials = { 1..6 }
            .into_par_iter()
            .map(|degree| {
                (
                    degree,
                    try_fit_poly_with_residual(x_values.view(), y_values.view(), degree)
                        .expect("Could not fit polynomial"),
                )
            })
            .collect::<Vec<_>>();

        polynomials.sort_by(
            |(_, PolyFit { residual, .. }),
             (
                _,
                PolyFit {
                    residual: residual2,
                    ..
                },
            )| {
                residual
                    .partial_cmp(residual2)
                    .expect("Could not compare residuals")
            },
        );

        let polynomial = polynomials.remove(0).1.polynomial;

        log::debug!("Done fitting polynomial to curve: {self:?}");

        polynomial
    }

    /// Start fitting a polynomial to the curve if not already started.
    /// Use the fitted polynomial if available or sample the curve with a binary search.
    pub fn value(&self, x: f32) -> f32 {
        let mut polynomials = POLYNOMIALS.lock();
        match polynomials.get(self) {
            Some(Some(polynomial)) => return polynomial.eval(x as f64) as f32,
            None => {
                POLYNOMIAL_QUEUE.send(*self).ok();
                polynomials.insert(*self, None);
            }
            Some(None) => (),
        }
        drop(polynomials);

        // polynomial is not yet available, sample the curve with a binary search
        let bezier = QuadraticBezierShape::from_points_stroke(
            [self.start, self.bezier, self.end],
            false,
            Color32::TRANSPARENT,
            Stroke::default(),
        );
        let mut n = 2;
        let mut t = 0.5;
        loop {
            let sample = bezier.sample(t);
            let e = sample.x - x;
            if e.abs() < 0.005 {
                break sample.y;
            }
            let step = 1.0 / 2.0f32.powi(n);
            if e < 0.0 {
                t += step;
            } else {
                t -= step;
            }
            n += 1;
        }
    }
}
