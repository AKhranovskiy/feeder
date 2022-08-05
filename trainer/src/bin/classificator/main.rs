mod classify;
mod config;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytes::Bytes;
use clap::Parser;
use config::ClassificationConfig;
use mfcc::ffmpeg_decode;
use ndarray::s;
use tch::IndexOp;
use trainer::networks::Network;
use trainer::utils::data::IMAGE_TENSOR_SHAPE;

use crate::classify::classify;

#[derive(Parser)]
#[clap(about = "Classify an audio segment.")]
struct Args {
    /// Network name.
    #[clap(arg_enum, short, long)]
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

    let raw_data: mfcc::RawAudioData = ffmpeg_decode(Bytes::from(buffer))?;
    let mfccs = calculate_mfccs(&raw_data)?;

    println!("Prepared {} images.", mfccs.len());

    let images = tch::Tensor::zeros(&[mfccs.len() as i64, 1, 39, 171], tch::kind::FLOAT_CPU);
    for (idx, m) in mfccs.iter().enumerate() {
        let image =
            tch::Tensor::try_from(m).map(|t| t.transpose(0, 1).reshape(&IMAGE_TENSOR_SHAPE))?;
        images.i(idx as i64).copy_(&image);
    }

    let (probabilities, timings) = classify(&args.network, &config, &images)?;

    let (_, classes) = probabilities.max_dim(1, false);
    let classes = Vec::<u8>::from(&classes);

    print!("ADVER ");
    for class in &classes {
        if class == &0 {
            print!("#")
        } else {
            print!(" ")
        }
    }
    println!(
        "  {:0.2}%",
        classes.iter().filter(|&x| x == &0).count() as f64 * 100.0 / classes.len() as f64
    );

    print!("MUSIC ");
    for class in &classes {
        if class == &1 {
            print!("#")
        } else {
            print!(" ")
        }
    }
    println!(
        "  {:0.2}%",
        classes.iter().filter(|&x| x == &1).count() as f64 * 100.0 / classes.len() as f64
    );

    print!("TALK  ");
    for class in &classes {
        if class == &2 {
            print!("#")
        } else {
            print!(" ")
        }
    }
    println!(
        "  {:0.2}%",
        classes.iter().filter(|&x| x == &2).count() as f64 * 100.0 / classes.len() as f64
    );

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
