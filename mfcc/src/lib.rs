use std::io::Write;
use std::iter::IntoIterator;
use std::ops::Mul;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Result};
use bytemuck::cast_slice;
use bytes::Bytes;
use itertools::Itertools;
use ndarray::{concatenate, Axis};
use ordered_float::OrderedFloat;
use plotters::prelude::{BitMapBackend, IntoDrawingArea};
use plotters::style::RGBColor;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

pub type RawAudioData = ndarray::Array1<f32>;
pub type MFCCs = ndarray::Array2<f32>;

pub const SAMPLE_RATE: u32 = 22050;
const FRAME_SIZE: usize = 512;
const N_FILTERS: usize = 40;
const N_COEFFS: usize = 13;

pub fn calculate_mel_coefficients_with_deltas(data: &RawAudioData) -> Result<MFCCs> {
    let mfccs = extract_mfccs(data)?;
    let delta = mfccs_delta(&mfccs);
    let delta2 = mfccs_delta(&delta);
    Ok(concatenate![Axis(1), mfccs, delta, delta2])
}

fn extract_mfccs(data: &RawAudioData) -> Result<MFCCs> {
    let mfccs = data
        .exact_chunks(FRAME_SIZE)
        .into_iter()
        .map(|chunk| {
            let mut mfcc = aubio::MFCC::new(FRAME_SIZE, N_FILTERS, N_COEFFS, SAMPLE_RATE)?;
            let mut output = [0f32; N_COEFFS];
            // TODO handle unwrap.
            mfcc.do_(chunk.as_slice().unwrap(), &mut output)
                .map(|_| output)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(IntoIterator::into_iter)
        .collect::<Vec<_>>();

    let mfccs = MFCCs::from_shape_vec((mfccs.len() / N_COEFFS, N_COEFFS), mfccs).unwrap();

    Ok(mfccs)
}

fn mfccs_delta(input: &MFCCs) -> MFCCs {
    let numrows = input.dim().0;

    let delta = (0..numrows)
        .map(|t| {
            (input.row(t.saturating_sub(2)).mul(-2.0)
                + input.row(t.saturating_sub(1)).mul(-1.0)
                + input.row(t.saturating_add(1).min(numrows - 1)).mul(1.0)
                + input.row(t.saturating_add(2).min(numrows - 1)).mul(2.0))
                / 10.0
        })
        .flat_map(IntoIterator::into_iter)
        .collect::<Vec<_>>();
    MFCCs::from_shape_vec(input.dim(), delta).unwrap()
}

const PLOT_FRAME_WIDTH: u32 = 2;
const PLOT_FRAME_HEIGHT: u32 = 10;

pub fn plot(data: &MFCCs, filename: &str) {
    let (min, max) = match data.iter().minmax_by_key(|&v| OrderedFloat(*v)) {
        itertools::MinMaxResult::NoElements => (0f32, 0f32),
        itertools::MinMaxResult::OneElement(v) => (*v, *v),
        itertools::MinMaxResult::MinMax(a, b) => (*a, *b),
    };

    // println!("Min={min}, Max={max}");

    let mut colors = colorgrad::spectral().colors(N_COEFFS);
    colors.reverse();

    let grad = colorgrad::CustomGradient::new()
        .colors(&colors)
        .domain(&[min.into(), max.into()])
        .build()
        .expect("failed to build gradient");

    let path = format!("plots/{filename}.bmp");
    let (width, height) = data.dim();

    let root = BitMapBackend::new(
        &path,
        (
            width as u32 * PLOT_FRAME_WIDTH,
            height as u32 * PLOT_FRAME_HEIGHT,
        ),
    )
    .into_drawing_area();

    let areas = root.split_evenly((height, width));
    for (area, index) in areas.into_iter().zip(0..) {
        let row_index = index % width;
        let col_index = height - index / width - 1;
        let value = data[[row_index, col_index]];

        let (r, g, b, _) = grad.at(value.into()).to_linear_rgba_u8();
        let color = RGBColor(r, g, b);

        area.fill(&color).expect("failed to fill area");
    }
}

pub fn ffmpeg_decode(bytes: Bytes) -> Result<RawAudioData> {
    let ffmpeg_path = std::env::var("FFMPEG_PATH")?;

    // println!("Execute ffmpeg, path={ffmpeg_path}");

    let mut proc = Command::new("ffmpeg")
        .env("PATH", ffmpeg_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .args(
            format!("-i pipe:0 -acodec pcm_s16le -ar {SAMPLE_RATE} -ac 1 -f wav -v fatal pipe:1")
                .split_ascii_whitespace(),
        )
        .spawn()?;

    let mut stdin = proc
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to get stdin"))?;

    std::thread::spawn(move || {
        stdin.write_all(&bytes).expect("Failed to write content");
    });

    let output = proc.wait_with_output()?.stdout;

    let data = cast_slice::<u8, i16>(output.as_ref())
        .into_par_iter()
        .map(|x| f32::from(*x))
        .collect::<Vec<_>>();

    Ok(RawAudioData::from_shape_vec(data.len(), data).unwrap())
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use ndarray::array;
    use ordered_float::OrderedFloat;

    #[test]
    fn test_min_max() {
        let sut = array![[1.0, 2.0], [-1.0, -2.0]];
        let mm = sut.iter().minmax_by_key(|&v| OrderedFloat(*v));
        println!("{mm:?}");
    }
}
