use std::time::Instant;

use kdam::term::Colorizer;
use kdam::tqdm;
use tch::nn::ModuleT;
use tch::vision::dataset::augmentation;
use trainer::utils::data::{load_data, prepare_dataset};
use trainer::utils::Stats;

use crate::config::TrainingConfig;
use crate::Network;

pub fn train(network: &Network, config: TrainingConfig) -> anyhow::Result<(Stats, Stats)> {
    let mut timings = Stats::new();
    let mut accuracy = Stats::new();

    let (images, labels) = load_data(&config.data_directory, &config.samples)?;

    let dataset = prepare_dataset(images, labels, config.test_fraction);

    let (vs, loaded) = network.create_varstore(&config.input_weights_filename);

    // TODO - hide into logger so it can be easily controlled by verbose flag.
    match loaded {
        Ok(_) => {
            print!("{}", "\tSuccess ".colorize("bold green"));
            println!(
                "Weights are loaded from {}.",
                config.input_weights_filename.display()
            );
        }
        Err(_) => {
            print!("{}", "\tFailure ".colorize("red"));
            println!(
                "Couldn't load weights from {}.",
                config.input_weights_filename.display()
            );
        }
    }

    // TODO - merge method, so it returns both network and optimizer.
    //        Although an optimizer could be choosen independently.
    let net = network.create_network(&vs.root());
    let mut opt = network.create_optimizer(&vs)?;

    const TRAIN_BATCH_SIZE: i64 = 64;
    const TEST_BATCH_SIZE: i64 = 512;

    let epoch_steps = dataset.train_images.size()[0] / TRAIN_BATCH_SIZE;

    for epoch in tqdm!(
        0..config.epochs,
        desc = "Training",
        animation = "fillup",
        unit = "epoch",
        force_refresh = true,
        position = 0
    ) {
        let timer_epoch = Instant::now();

        opt.set_lr(network.learning_rate(epoch));
        for (bimages, blabels) in tqdm!(
            dataset
                .train_iter(TRAIN_BATCH_SIZE)
                .shuffle()
                .to_device(vs.device()),
            total = epoch_steps as usize,
            desc = "Batches",
            unit = "batch",
            force_refresh = true,
            position = 1
        ) {
            if config.dry_run {
                continue;
            }

            let bimages = augmentation(&bimages, false, 4, 8);
            let loss = net
                .forward_t(&bimages, true)
                .cross_entropy_for_logits(&blabels);
            opt.backward_step(&loss);
        }

        let timer_epoch_duration = timer_epoch.elapsed().as_secs_f64();
        timings = timings.push(timer_epoch_duration);

        if config.dry_run {
            continue;
        }

        let test_accuracy = net.batch_accuracy_for_logits(
            &dataset.test_images,
            &dataset.test_labels,
            vs.device(),
            TEST_BATCH_SIZE,
        );

        accuracy = accuracy.push(test_accuracy);
    }

    // TODO - store a copy with extended name.
    vs.save(&config.output_weights_filename)?;

    // Preserve space for progress bars.
    println!();
    println!();

    Ok((timings, accuracy))
}
