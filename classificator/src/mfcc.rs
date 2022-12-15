use crate::config::{MFCC_FRAME_SIZE, MFCC_HOP_LENGTH, MFCC_N_COEFFS, MFCC_N_FILTERS, SAMPLE_RATE};
use crate::util::{stepped_window_ranges, stepped_windows};

/// Calculates series of Mel-frequency cepstral coefficients (MFCC).
pub async fn calculate(input: &[f32]) -> anyhow::Result<Vec<f32>> {
    let (_, tail) = stepped_windows(input.len(), MFCC_FRAME_SIZE, MFCC_HOP_LENGTH);

    let input = if tail != 0 {
        let mut v = input.to_vec();
        v.extend_from_within(v.len() - tail..);
        v
    } else {
        input.to_vec()
    };

    let (segments, tail) = stepped_windows(input.len(), MFCC_FRAME_SIZE, MFCC_HOP_LENGTH);

    assert_eq!(tail, 0);

    let mut output = Vec::<f32>::with_capacity(segments * MFCC_N_COEFFS);
    let mut mfcc = aubio::MFCC::new(
        MFCC_FRAME_SIZE,
        MFCC_N_FILTERS,
        MFCC_N_COEFFS,
        SAMPLE_RATE as u32,
    )?;

    for r in stepped_window_ranges(input.len(), MFCC_FRAME_SIZE, MFCC_HOP_LENGTH) {
        let chunk = &input[r];
        let mut mfccs = [0f32; MFCC_N_COEFFS];

        #[allow(clippy::needless_borrow)]
        mfcc.do_(chunk, &mut mfccs)?;

        output.extend_from_slice(&mfccs);
    }

    Ok(output)
}
