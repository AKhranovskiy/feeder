#![feature(const_trait_impl)]

use std::time::Duration;

use ndarray::{Array2, Axis};

use crate::util::stepped_windows;

use self::util::stepped_window_ranges;

mod delta;
mod util;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub sample_rate_hz: usize,
    pub num_coefficients: usize,
    pub frame_size: usize,
    pub hop_length: usize,
    pub deltas: bool,
}

impl Config {
    #[must_use]
    pub const fn frame_duration(&self) -> Duration {
        let ms = self.frame_size * 1000 / self.sample_rate_hz;
        Duration::from_millis(ms as u64)
    }
}

impl const Default for Config {
    fn default() -> Self {
        Self {
            sample_rate_hz: 22050,
            num_coefficients: 39,
            deltas: false,
            // FFT 512 is recommended by librosa for speech processing
            // FFT 2048 is used by PP CNN.
            frame_size: 441,
            hop_length: 220,
        }
    }
}

const FILTERS: usize = 40; // Aubio says it must be 40 for MFCC

pub fn calculate_mfccs(input: &[f32], config: Config) -> anyhow::Result<ndarray::Array2<f64>> {
    assert!(
        config.num_coefficients % 3 == 0,
        "Coeff must be multipler of 3"
    );

    let coeff = config.num_coefficients;

    let (segments, _) = stepped_windows(input.len(), config.frame_size, config.hop_length);

    let mut pvoc = aubio::PVoc::new(config.frame_size, config.hop_length)?;
    pvoc.set_window(aubio::WindowType::Hanningz)?;

    let mut mfcc = aubio::MFCC::new(
        config.frame_size,
        FILTERS,
        coeff,
        config.sample_rate_hz as u32,
    )?;

    let mut output = Array2::<f64>::from_elem((segments, coeff), 0.0);

    for r in stepped_window_ranges(input.len(), config.frame_size, config.hop_length) {
        let chunk = &input[r];

        // let mut fftgrain = vec![0.0f32; config.frame_size];
        // pvoc.do_(chunk, fftgrain.as_mut_slice())?;

        let mut buf = vec![0f32; coeff];
        mfcc.do_(chunk, buf.as_mut_slice())?;

        assert!(buf.iter().all(|x| x.is_finite()), "MFCC are finite");

        output.append(
            Axis(0),
            Array2::from_shape_vec((1, coeff), buf)?
                .mapv(f64::from)
                .view(),
        )?;
    }
    // let d = deltas(output);
    // assert_eq!(config.num_coefficients, d.shape()[1]);
    // Ok(d)
    Ok(output)
}
