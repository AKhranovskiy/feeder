use std::env::args;
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter, Write};

use anyhow::ensure;
use bytemuck::cast_slice_mut;
use codec::{CodecParams, Decoder, Encoder, SampleFormat};

fn main() -> anyhow::Result<()> {
    let file_a = args().nth(1).expect("Expects file");
    let file_b = args().nth(2).expect("Expects file");

    let mut decoder_a = Decoder::try_from(BufReader::new(File::open(file_a)?))?;
    let mut decoder_b = Decoder::try_from(BufReader::new(File::open(file_b)?))?;

    ensure!(
        decoder_a.codec_params() == decoder_b.codec_params(),
        "Audio format mismatches: {:?} != {:?}",
        decoder_a.codec_params(),
        decoder_b.codec_params()
    );

    let mut encoder = Encoder::opus(decoder_a.codec_params(), BufWriter::new(stdout()))?;

    // 576 samples per frame.
    // ~38 frames per second

    let cf = cross_fade_coeffs(576 * 38 * 3); // ~3 secs
                                              //
    let frames_a = decoder_a.collect::<anyhow::Result<Vec<_>>>()?;
    let frames_b = decoder_b.collect::<anyhow::Result<Vec<_>>>()?;

    let mut frames = Vec::with_capacity(frames_a.len() / 2 + frames_b.len() * 2 + 38 * 3);

    frames.extend_from_slice(&frames_a[..frames_a.len() / 2]);

    // todo cross-fade

    frames.extend_from_slice(&frames_b[frames_b.len() / 2..]);

    for frame in frames {
        encoder.push(frame)?;
    }

    encoder.flush()?;

    Ok(())
}

fn cross_fade(enter: &[f32], exit: &[f32]) -> Vec<f32> {
    assert_eq!(enter.len(), exit.len());

    exit.into_iter()
        .zip(enter.into_iter())
        .zip(cross_fade_coeffs(enter.len()).into_iter())
        .map(|((exit, enter), (fin, fout))| (exit * fin).max(enter * fout))
        .collect()
}

fn cross_fade_coeffs(size: usize) -> Vec<(f32, f32)> {
    // https://signalsmith-audio.co.uk/writing/2021/cheap-energy-crossfade/

    let step = 1.0f64 / (size - 1) as f64;

    (0..size)
        .map(|n| {
            let x = step * (n as f64);
            let x2 = 1_f64 - x;
            let a = x * x2;
            let b = a + 1.4186_f64 * a.powi(2);
            let fin = (b + x).powi(2) as f32;
            let fout = (b + x2).powi(2) as f32;
            (fin, fout)
        })
        .collect()
}
