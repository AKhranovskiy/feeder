mod config;
mod train;

use std::path::PathBuf;

use clap::Parser;
use trainer::networks::Network;

use crate::config::TrainingConfig;
use crate::train::train;

#[derive(Parser)]
#[clap(about = "Trains a network using given samples.")]
pub struct Args {
    /// Network name.
    #[clap(arg_enum, short, long)]
    network: Network,

    /// Input weight file. Default "model.[NETWORK NAME].tch".
    #[clap(value_parser, short, long)]
    input_weights: Option<String>,

    /// Output weight file. Default "model.[NETWORK NAME].tch".
    #[clap(value_parser, short, long)]
    output_weights: Option<String>,

    /// Data (bins) directory.
    #[clap(short, long, value_parser, value_name = "DATA_DIR", default_value_t = String::from("./bins/"))]
    data: String,

    /// Number of samples for training, all samples by default.
    #[clap(long, short)]
    samples: Option<usize>,

    /// Test fraction, in percents (0..50).
    #[clap(short, long, default_value_t = 25, value_parser=clap::value_parser!(u8).range(0..50))]
    test_fraction: u8,

    /// Number of epochs.
    #[clap(short, long, default_value_t = 150)]
    epochs: usize,

    /// Simulate training process, for development purposes.
    #[clap(long, default_value_t = false, action)]
    dry_run: bool,
}

impl From<&Args> for TrainingConfig {
    fn from(args: &Args) -> Self {
        let default_weights_filename = format!("model.{:?}.tch", &args.network);

        Self {
            input_weights_filename: PathBuf::from(
                args.input_weights
                    .as_ref()
                    .unwrap_or(&default_weights_filename),
            ),
            output_weights_filename: PathBuf::from(
                args.output_weights
                    .as_ref()
                    .unwrap_or(&default_weights_filename),
            ),
            data_directory: PathBuf::from(&args.data),
            samples: args.samples,
            test_fraction: f64::from(args.test_fraction) / 100.0,
            epochs: args.epochs,
            dry_run: args.dry_run,
        }
    }
}

fn main() -> anyhow::Result<()> {
    tch::set_num_threads(num_cpus::get() as i32);

    let args = Args::parse();

    let (timings, accuracy) = train(&args.network, TrainingConfig::from(&args))?;

    println!(
        "Final accuracy: {:.02}%, min/max/avg={:0.2}%/{:.02}%/{:.02}%",
        accuracy.last().unwrap_or_default() * 100.0,
        accuracy.min().unwrap_or_default() * 100.0,
        accuracy.max().unwrap_or_default() * 100.0,
        accuracy.avg().unwrap_or_default() * 100.0
    );

    println!(
        "Elapsed: {:.02}s. Epoch time: min/max/avg={:0.2}s/{:.02}s/{:.02}s",
        timings.sum().unwrap_or_default(),
        timings.min().unwrap_or_default(),
        timings.max().unwrap_or_default(),
        timings.avg().unwrap_or_default()
    );

    Ok(())
}
