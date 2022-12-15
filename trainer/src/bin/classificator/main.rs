mod classify;
mod config;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use clap::Parser;
use config::ClassificationConfig;
use mfcc::ffmpeg_decode;
use ndarray::s;
use tch::IndexOp;
use trainer::networks::Network;
use trainer::utils::Stats;

use crate::classify::classify;

#[derive(Parser)]
#[clap(about = "Classify an audio segment.")]
struct Args {
    /// Network name.
    #[clap(value_enum, short, long)]
    network: Network,

    /// Input weight file. Default "model.[NETWORK NAME].tch".
    #[clap(value_parser, short, long)]
    input_weights: Option<String>,

    /// Input audio file.
    #[clap(short, long)]
    file: String,

    /// Simulate classificaton process, for development purposes.
    #[clap(long, default_value_t = false, action)]
    dry_run: bool,
}

impl From<&Args> for ClassificationConfig {
    fn from(args: &Args) -> Self {
        let default_weights_filename = format!("model.{:?}.tch", &args.network);

        Self {
            input_weights_filename: PathBuf::from(
                args.input_weights
                    .as_ref()
                    .unwrap_or(&default_weights_filename),
            ),
            audio_file: PathBuf::from(&args.file),
            dry_run: args.dry_run,
        }
    }
}

fn main() -> anyhow::Result<()> {
    tch::set_num_threads(num_cpus::get() as i32);

    let args = Args::parse();

    let config = ClassificationConfig::from(&args);

    let mut buffer = Vec::new();
    File::open(&config.audio_file)?.read_to_end(&mut buffer)?;

    let raw_data: mfcc::RawAudioData = ffmpeg_decode(&buffer)?;
    let mfccs = calculate_mfccs(&raw_data)?;

    println!("Prepared {} images.", mfccs.len());

    let images = tch::Tensor::zeros(&[mfccs.len() as i64, 1, 39, 171], tch::kind::FLOAT_CPU);
    for (idx, m) in mfccs.iter().enumerate() {
        let image = tch::Tensor::try_from(m).map(|t| t.transpose(0, 1).reshape(&[1, 39, 171]))?;
        images.i(idx as i64).copy_(&image);
    }

    let (probabilities, timings) = classify(&args.network, &config, &images)?;

    print_score("Simple Max    ", &simple_max_score(&probabilities));
    print_score("Moving Average", &moving_avg_score(&probabilities));
    print_score("Timed Average ", &avg_timed_score(&probabilities));

    println!(
        "Elapsed: {:.02}s. Chunk time: min/max/avg={:0.2}s/{:.02}s/{:.02}s",
        timings.sum().unwrap_or_default(),
        timings.min().unwrap_or_default(),
        timings.max().unwrap_or_default(),
        timings.avg().unwrap_or_default()
    );
    Ok(())
}

fn calculate_mfccs(data: &mfcc::RawAudioData) -> anyhow::Result<Vec<mfcc::MFCCs>> {
    let window_size = mfcc::SAMPLE_RATE as usize * 4; // 4 secs
    let window_step = mfcc::SAMPLE_RATE as usize; // 1 sec
    let steps = ((data.len() - window_size) as f32 / window_step as f32).round() as usize + 1;
    let mut mels = Vec::new();
    for step in 0..steps {
        let window_start = step * window_step;
        let window_end = (window_start + window_size).min(data.len());
        let data = data.slice(s![window_start..window_end]).to_owned();
        let mfccs = mfcc::calculate_mel_coefficients_with_deltas(&data)?;
        // The row number varies around 171/172, so lets take the least.`
        let mfccs = mfccs.slice(s![..171, ..]).to_owned();
        mels.push(mfccs);
    }
    Ok(mels)
}

fn print_score(label: &str, classes: &[u8]) {
    println!(
        "{label} {} ({})",
        classes
            .iter()
            .map(|x| match x {
                0 => 'A',
                1 => 'M',
                2 => 'T',
                _ => ' ',
            })
            .collect::<String>(),
        classes.len()
    )
}
fn simple_max_score(probabilities: &tch::Tensor) -> Vec<u8> {
    let (_, classes) = probabilities.max_dim(1, false);
    Vec::<u8>::from(&classes)
}

fn moving_avg_score(probabilities: &tch::Tensor) -> Vec<u8> {
    Vec::<Vec<f32>>::from(probabilities)
        .windows(4)
        .map(|w| {
            let a = scalar_mul(&w[0], 0.25);
            let b = scalar_mul(&w[1], 0.5);
            let c = scalar_mul(&w[2], 0.75);
            let d = scalar_mul(&w[3], 1.0);

            let sum = [a, b, c, d]
                .into_iter()
                .reduce(|accum, v| vector_sum(&accum, &v))
                .unwrap_or_default();

            max_index(&sum).unwrap_or(u8::MAX as usize) as u8
        })
        .collect()
}

fn scalar_mul(v: &[f32], scalar: f32) -> Vec<f32> {
    v.iter().map(|x| x * scalar).collect()
}

fn vector_sum(a: &[f32], b: &[f32]) -> Vec<f32> {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
}

fn avg_timed_score(probabilities: &tch::Tensor) -> Vec<u8> {
    let probabilities = Vec::<Vec<f32>>::from(probabilities);

    let n = probabilities.len();
    assert!(n > 0);

    (0..n + 3)
        .map(|sec| {
            let a = sec.checked_sub(3).unwrap_or_default();
            let b = sec.min(n - 1);

            let stats = (a..=b).map(|idx| &probabilities[idx]).fold(
                (Stats::new(), Stats::new(), Stats::new()),
                |accum, values| {
                    (
                        accum.0.push(values[0] as f64),
                        accum.1.push(values[1] as f64),
                        accum.2.push(values[2] as f64),
                    )
                },
            );
            max_index(&[
                stats.0.avg().unwrap_or_default(),
                stats.1.avg().unwrap_or_default(),
                stats.2.avg().unwrap_or_default(),
            ])
            .unwrap_or(u8::MAX as usize) as u8
        })
        .collect()
}

fn max_index<T>(v: &[T]) -> Option<usize>
where
    T: PartialOrd + Copy,
{
    let mut max_value = None;
    let mut max_index = None;
    for (idx, &value) in v.iter().enumerate() {
        match max_value {
            None => {
                max_value = Some(value);
                max_index = Some(idx);
            }
            Some(max) => {
                if value > max {
                    max_value = Some(value);
                    max_index = Some(idx);
                }
            }
        }
    }
    max_index
}
