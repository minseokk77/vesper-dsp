use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type};
use crate::shared_state::{VesperApoSharedState, MAX_EQ_BANDS};
use std::sync::atomic::Ordering;

pub struct VesperApoDspPipeline {
    sample_rate: f64,
    left_filters: Vec<DirectForm1<f64>>,
    right_filters: Vec<DirectForm1<f64>>,
    preamp_linear: f64,
    last_seq: u64,
}

impl VesperApoDspPipeline {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            left_filters: Vec::with_capacity(MAX_EQ_BANDS),
            right_filters: Vec::with_capacity(MAX_EQ_BANDS),
            preamp_linear: 1.0,
            last_seq: u64::MAX,
        }
    }

    pub fn update_from_shared_state(&mut self, state: &VesperApoSharedState) {
        let current_seq = state.update_sequence.load(Ordering::Relaxed);
        if self.last_seq == current_seq {
            return;
        }
        self.last_seq = current_seq;

        self.left_filters.clear();
        self.right_filters.clear();
        self.preamp_linear = 10_f64.powf(state.preamp_gain_db as f64 / 20.0);

        let band_count = (state.band_count as usize).min(MAX_EQ_BANDS);
        for i in 0..band_count {
            let band = &state.bands[i];
            let filter_type = match band.filter_type {
                1 => Type::LowShelf(band.gain_db as f64),
                2 => Type::HighShelf(band.gain_db as f64),
                3 => Type::LowPass,
                4 => Type::HighPass,
                5 => Type::Notch,
                _ => Type::PeakingEQ(band.gain_db as f64),
            };

            let freq = (band.frequency as f64).clamp(20.0, (self.sample_rate * 0.49).max(20.0));
            let q = (band.q as f64).clamp(0.1, 10.0);

            if let Ok(coeff) = Coefficients::<f64>::from_params(filter_type, self.sample_rate.hz(), freq.hz(), q) {
                self.left_filters.push(DirectForm1::<f64>::new(coeff));
                self.right_filters.push(DirectForm1::<f64>::new(coeff));
            }
        }
    }

    pub fn process_interleaved_stereo(&mut self, buffer: &mut [f32]) {
        let len = buffer.len();
        if len < 2 {
            return;
        }

        let mut i = 0;
        while i + 1 < len {
            let mut left = buffer[i] as f64 * self.preamp_linear;
            let mut right = buffer[i + 1] as f64 * self.preamp_linear;

            for filter in &mut self.left_filters {
                left = filter.run(left);
            }
            for filter in &mut self.right_filters {
                right = filter.run(right);
            }

            buffer[i] = left.clamp(-1.0, 1.0) as f32;
            buffer[i + 1] = right.clamp(-1.0, 1.0) as f32;
            i += 2;
        }
    }
}
