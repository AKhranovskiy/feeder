use std::env::args;
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter, Write};

use anyhow::ensure;
use bytemuck::{cast_slice, cast_slice_mut};
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

    let cf = cross_fade_coeffs(576 * 38 * 4); // ~3 secs
                                              //
    let frames_a = decoder_a.collect::<anyhow::Result<Vec<_>>>()?;
    let frames_b = decoder_b.collect::<anyhow::Result<Vec<_>>>()?;

    let mut frames = Vec::with_capacity(frames_a.len() / 2 + frames_b.len() * 2 + 38 * 4);

    frames.extend_from_slice(&frames_a[..frames_a.len() / 2]);

    // todo cross-fade
    for (index, (a, b)) in frames_a[frames_a.len() / 2..][0..38 * 3]
        .iter()
        .zip(frames_b[..frames_b.len() / 2][0..38 * 4].iter())
        .enumerate()
    {
        let planes_a = a.planes();
        assert_eq!(1, planes_a.len());

        let planes_b = b.planes();
        assert_eq!(1, planes_b.len());

        let data_a = cast_slice::<_, f32>(planes_a[0].data());
        let data_b = cast_slice::<_, f32>(planes_b[0].data());

        let mut frame = codec::silence_frame(&a).into_mut();
        let mut planes = frame.planes_mut();
        let mut data = cast_slice_mut::<_, f32>(planes[0].data_mut());

        for (((a, b), c), d) in data_a
            .iter()
            .zip(data_b.iter())
            .zip(cf[index * 576..][..576].iter())
            .zip(data.iter_mut())
        {
            *d = (a * c.1).max(b * c.0)
        }

        frames.push(frame.freeze());
    }

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
