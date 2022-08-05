use std::time::Instant;

use kdam::term::Colorizer;
use kdam::tqdm;
use tch::nn::ModuleT;
use tch::vision::dataset::{augmentation, Dataset};
use trainer::utils::data::{load_data, prepare_dataset};
use trainer::utils::Stats;

use crate::config::TrainingConfig;
use crate::Network;

pub fn train(network: &Network, config: TrainingConfig) -> anyhow::Result<(Stats, Stats)> {
    let mut timings = Stats::new();
    let mut accuracy = Stats::new();

    let (images, labels) = load_data(&config.data_directory, &config.samples)?;

    let dataset = prepare_dataset(images, labels, config.test_fraction);

    show_class_statistic(&dataset);

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

fn show_class_statistic(ds: &Dataset) {
    let count = |x: i16, v: &Vec<i16>| -> (usize, f64) {
        let cnt = v.iter().filter(|&x_| x_ == &x).count();
        (cnt, cnt as f64 * 100.0 / v.len() as f64)
    };

    let train_classes = Vec::<i16>::from(&ds.train_labels);
    let train_ads = count(0, &train_classes);
    let train_music = count(1, &train_classes);
    let train_talks = count(2, &train_classes);

    println!(
        "Train set: ads {}/{:.02}, music {}/{:.02}, talks {}/{:.02}",
        train_ads.0, train_ads.1, train_music.0, train_music.1, train_talks.0, train_talks.1
    );

    let test_classes = Vec::<i16>::from(&ds.test_labels);
    let test_ads = count(0, &test_classes);
    let test_music = count(1, &test_classes);
    let test_talks = count(2, &test_classes);

    println!(
        "Test set: ads {}/{:.02}, music {}/{:.02}, talks {}/{:.02}",
        test_ads.0, test_ads.1, test_music.0, test_music.1, test_talks.0, test_talks.1
    );
}
