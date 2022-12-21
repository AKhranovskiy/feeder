use crate::util::stepped_windows;

use self::util::stepped_window_ranges;

mod util;

#[allow(dead_code)]
pub struct Config {
    sample_rate_hz: usize,
    num_coefficients: usize,
    frame_size: usize,
    hop_length: usize,
    filters: usize,
    deltas: bool,
    extend_tail: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sample_rate_hz: 22050,
            num_coefficients: 39,
            deltas: false,
            // FFT 512 is recommended by librosa for speech processing
            // FFT 2048 is used by PP CNN.
            frame_size: 441, // 20ms
            hop_length: 220, // 10ms
            filters: 40,
            extend_tail: false,
        }
    }
}

pub fn calculate_mfccs(input: &[f32], config: Config) -> anyhow::Result<ndarray::Array2<f32>> {
    // let (_, tail) = stepped_windows(input.len(), config.frame_size, config.hop_length);
    // let input = if tail != 0 {
    //     let mut v = input.to_vec();
    //     v.extend_from_within(v.len() - tail..);
    //     v
    // } else {
    //     input.to_vec()
    // };

    let (segments, _) = stepped_windows(input.len(), config.frame_size, config.hop_length);

    let mut output = Vec::<f32>::with_capacity(segments * config.num_coefficients);
    let mut mfcc = aubio::MFCC::new(
        config.frame_size,
        config.filters,
        config.num_coefficients,
        config.sample_rate_hz as u32,
    )?;

    for r in stepped_window_ranges(input.len(), config.frame_size, config.hop_length) {
        let chunk = &input[r];
        let mut mfccs = Vec::<f32>::new();
        mfccs.resize(config.num_coefficients, 0f32);

        #[allow(clippy::needless_borrow)]
        mfcc.do_(chunk, &mut mfccs)?;

        output.extend_from_slice(&mfccs);
    }

    let output = ndarray::Array2::from_shape_vec((segments, config.num_coefficients), output)?;
    Ok(output)
}
