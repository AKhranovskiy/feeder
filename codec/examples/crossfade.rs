use std::env::args;
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter};

use anyhow::ensure;
use bytemuck::{cast_slice, cast_slice_mut};
use codec::{Decoder, Encoder};

fn main() -> anyhow::Result<()> {
    let file_a = args().nth(1).expect("Expects file");
    let file_b = args().nth(2).expect("Expects file");

    let decoder_a = Decoder::try_from(BufReader::new(File::open(file_a)?))?;
    let decoder_b = Decoder::try_from(BufReader::new(File::open(file_b)?))?;

    ensure!(
        decoder_a.codec_params() == decoder_b.codec_params(),
        "Audio format mismatches: {:?} != {:?}",
        decoder_a.codec_params(),
        decoder_b.codec_params()
    );

    let mut encoder = Encoder::opus(decoder_a.codec_params(), BufWriter::new(stdout()))?;

    let frames_a = decoder_a.collect::<anyhow::Result<Vec<_>>>()?;
    let frames_b = decoder_b.collect::<anyhow::Result<Vec<_>>>()?;

    let len = frames_a.len().min(frames_b.len());

    // 576 samples per frame.

    let cf = cross_fade_coeffs(576 * len); // ~3 secs
                                           //
    let mut frames = Vec::with_capacity(frames_a.len().max(frames_b.len()));

    for index in 0..len {
        let a = &frames_a[index];
        let planes_a = a.planes();
        assert_eq!(1, planes_a.len());

        let b = &frames_b[index];
        let planes_b = b.planes();
        assert_eq!(1, planes_b.len());

        let data_a = cast_slice::<_, f32>(planes_a[0].data());
        let data_b = cast_slice::<_, f32>(planes_b[0].data());

        let mut frame = a.clone().into_mut();
        let mut planes = frame.planes_mut();
        let data = cast_slice_mut::<_, f32>(planes[0].data_mut());

        for x in 0..576 {
            let (fout, fin) = cf[index * 576 + x];
            let sout = data_a[x];
            let sin = data_b[x];

            data[x] = (sout * fout).max(sin * fin);
        }

        frames.push(frame.freeze());
    }

    if frames_b.len() > len {
        frames.extend_from_slice(&frames_b[len..]);
    }

    for frame in frames {
        encoder.push(frame)?;
    }

    encoder.flush()?;

    Ok(())
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
            (fout, fin)
        })
        .collect()
}
