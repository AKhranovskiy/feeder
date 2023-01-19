use std::env::args;
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter};
use std::time::Duration;

use anyhow::ensure;
use bytemuck::{cast_slice, cast_slice_mut};
use codec::{CossinCrossFade, CrossFade, CrossFadePair, Decoder, Encoder};

fn main() -> anyhow::Result<()> {
    let file_in = args().nth(1).expect("Expects file");
    let file_out = args().nth(2).expect("Expects file");

    let decoder_in = Decoder::try_from(BufReader::new(File::open(file_in)?))?;
    let decoder_out = Decoder::try_from(BufReader::new(File::open(file_out)?))?;

    ensure!(
        decoder_in.codec_params() == decoder_out.codec_params(),
        "Audio format mismatches: {:?} != {:?}",
        decoder_in.codec_params(),
        decoder_out.codec_params()
    );

    let sr = decoder_in.codec_params().sample_rate() as usize;

    let mut encoder = Encoder::opus(decoder_in.codec_params(), BufWriter::new(stdout()))?;

    let frames_in = decoder_in.collect::<anyhow::Result<Vec<_>>>()?;
    let frames_out = decoder_out.collect::<anyhow::Result<Vec<_>>>()?;

    // 576 samples per frame.
    // ~38 frames per second
    let spf = frames_in[0].samples();
    let cross_fade_frames = ((3 * sr) as f64 / spf as f64).ceil().trunc() as usize;

    eprintln!(
        "left: {} frames, {} samples, {:0.03} secs",
        frames_in.len(),
        frames_in.len() * spf,
        (frames_in.len() * spf) as f64 / sr as f64
    );
    eprintln!(
        "right: {} frames, {} samples, {:0.03} secs",
        frames_out.len(),
        frames_out.len() * spf,
        (frames_out.len() * spf) as f64 / sr as f64
    );
    eprintln!(
        "cross-fade: {} frames, {} samples, {:0.03} secs",
        cross_fade_frames,
        cross_fade_frames * spf,
        (cross_fade_frames * spf) as f64 / sr as f64
    );

    for frame in &frames_in[..frames_in.len() - cross_fade_frames] {
        encoder.push(frame.clone())?;
    }

    {
        let cf = cross_fade_coeffs(cross_fade_frames * spf);

        let left = &frames_in[frames_in.len() - cross_fade_frames..];
        let right = &frames_out[..cross_fade_frames];

        assert_eq!(cf.len(), left.len() * spf);
        assert_eq!(cf.len(), right.len() * spf);

        for index in 0..cross_fade_frames {
            let left_planes = left[index].planes();
            let left_data = cast_slice::<_, f32>(left_planes[0].data());

            let right_planes = right[index].planes();
            let right_data = cast_slice::<_, f32>(right_planes[0].data());

            let mut frame = left[index].clone().into_mut();
            let mut planes = frame.planes_mut();
            let data = cast_slice_mut::<_, f32>(planes[0].data_mut());

            for x in 0..spf {
                let c = cf[index * spf + x];
                let sout = left_data[x] as f64;
                let sin = right_data[x] as f64;

                data[x] = c.apply(sout, sin) as f32;
            }

            encoder.push(frame.freeze())?;
        }
    }

    let pts_shift = Duration::from_nanos(
        frames_in
            .last()
            .and_then(|f| f.pts().as_nanos())
            .unwrap_or_default() as u64
            - frames_out[cross_fade_frames]
                .pts()
                .as_nanos()
                .unwrap_or_default() as u64,
    );

    for frame in &frames_out[cross_fade_frames..] {
        let pts = frame.pts() + pts_shift;
        encoder.push(frame.clone().with_pts(pts))?;
    }

    encoder.flush()?;

    Ok(())
}

fn cross_fade_coeffs(size: usize) -> Vec<CrossFadePair> {
    CossinCrossFade::generate(size)
}
