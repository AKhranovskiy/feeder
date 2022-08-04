mod config;
mod predict;

use std::path::PathBuf;

use clap::Parser;
use config::PredictionConfig;
use trainer::networks::Network;

use crate::predict::predict;

#[derive(Parser)]
#[clap(about = "Predicts a content kind.")]
pub struct Args {
    /// Network name.
    #[clap(arg_enum, short, long)]
    network: Network,

    /// Weight file. Default "model.[NETWORK NAME].tch".
    #[clap(value_parser, short, long)]
    weights: Option<String>,

    /// Data (bins) directory.
    #[clap(short, long, value_parser, value_name = "DATA_DIR", default_value_t = String::from("./bins/"))]
    data: String,

    /// Number of samples for prediction.
    #[clap(long, short)]
    samples: Option<usize>,

    /// Simulate prediction process, for development purposes.
    #[clap(long, default_value_t = false, action)]
    dry_run: bool,
}

impl From<&Args> for PredictionConfig {
    fn from(args: &Args) -> Self {
        let default_weights_filename = format!("model.{:?}.tch", &args.network);
        Self {
            input_weights_file: PathBuf::from(
                args.weights.as_ref().unwrap_or(&default_weights_filename),
            ),
            data_directory: PathBuf::from(&args.data),
            samples: args.samples,
            dry_run: args.dry_run,
        }
    }
}

fn main() -> anyhow::Result<()> {
    tch::set_num_threads(num_cpus::get() as i32);

    let args = Args::parse();

    // predicted is tensor Nx3, where N is number of samples.
    // original is tensor N, where N is number of samples, contains original labels.
    // Print result by rows, compare to labels, calculate accuracy on prediction.
    // Calculate accuracy per label.

    // TODO - Select samples here and pass only images.
    let (predicted, original, timings) = predict(&args.network, PredictionConfig::from(&args))?;

    let (total, correct, accuracy) = accuracy(&original, &predicted);
    println!(
        "Predicted {correct} out of {total}. Accuracy: {:.02}%",
        accuracy * 100.0
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

fn accuracy(original: &tch::Tensor, predicted: &tch::Tensor) -> (usize, usize, f64) {
    let (_, predicted) = predicted.max_dim(1, false);
    let correct_predictions: f64 = original.eq_tensor(&predicted).sum(tch::Kind::Float).into();
    let accuracy = correct_predictions / original.size()[0] as f64;

    (
        original.size()[0] as usize,
        correct_predictions as usize,
        accuracy,
    )
}
