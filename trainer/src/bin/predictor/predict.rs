use std::time::Instant;

use anyhow::bail;
use kdam::term::Colorizer;
use kdam::tqdm;
use tch::nn::ModuleT;
use trainer::networks::Network;
use trainer::utils::data::load_data;
use trainer::utils::Stats;

use crate::config::PredictionConfig;

pub fn predict(
    network: &Network,
    config: PredictionConfig,
) -> anyhow::Result<(tch::Tensor, tch::Tensor, Stats)> {
    let mut timings = Stats::new();

    let (images, labels) = load_data(&config.data_directory, &config.samples)?;

    let (vs, loaded) = network.create_varstore(&config.input_weights_file);

    // TODO - hide into logger so it can be easily controlled by verbose flag.
    match loaded {
        Ok(_) => {
            print!("{}", "\tSuccess ".colorize("bold green"));
            println!(
                "Weights are loaded from {}.",
                config.input_weights_file.display()
            );
        }
        Err(_) => {
            print!("{}", "\tFailure ".colorize("red"));
            println!(
                "Couldn't load weights from {}.",
                config.input_weights_file.display()
            );
            bail!(
                "Failed to load weights from {}.",
                config.input_weights_file.display()
            );
        }
    }

    let net = network.create_network(&vs.root());

    let mut predictions = Vec::with_capacity(images.size()[0] as usize);

    for image in tqdm!(
        images.split(1, 0).iter(),
        desc = "Predicting",
        animation = "fillup",
        unit = "image",
        disable = false
    ) {
        let timer = Instant::now();

        predictions.push(
            net.forward_t(image, /*train=*/ false)
                .softmax(-1, tch::Kind::Float),
        );

        timings = timings.push(timer.elapsed().as_secs_f64());
    }

    let predictions = tch::Tensor::concat(&predictions, 0);

    Ok((predictions, labels, timings))
}
