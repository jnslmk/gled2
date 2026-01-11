use crate::audio::adsr::{Adsr, AdsrParams, LowPass};
use crate::audio::state::FftSample;


pub struct ReactiveSignal{
    pub adsr: Adsr,
    pub low_pass: LowPass,
    pub impulse: f32,
    pub output: f32,
}


impl ReactiveSignal{
    pub fn new(sample_rate: f32) -> Self{
        Self{
            adsr: Adsr::new(AdsrParams::default(), sample_rate),
            low_pass: LowPass::new(400., 200., sample_rate),
            impulse: 0.,
            output: 0.,
        }
    }

    pub fn tick(&mut self, input: &FftSample){
        self.impulse = (self.low_pass.tick(input)).log2().clamp(0.0, 1.0);
        self.output = self.adsr.tick(self.impulse);
    }
}
