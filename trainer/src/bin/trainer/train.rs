use std::time::Instant;

use kdam::{tqdm, BarExt};
use tch::nn::ModuleT;
use tch::vision::dataset::{Dataset};
use tch::IndexOp;
use trainer::utils::data::{load_data, prepare_dataset};
use trainer::utils::{Stats, ensure_finite};

use crate::config::TrainingConfig;
use crate::Network;

pub fn train(network: &Network, config: TrainingConfig) -> anyhow::Result<(Stats, Stats)> {
    let mut timings = Stats::new();
    let mut accuracy = Stats::new();

    let images = load_data(&config.input)?;
    let dataset = prepare_dataset(images, config.test_fraction);

    show_class_statistic(&dataset);

    crate::plot(&dataset.train_images.i(0));

    let vs = network.create_varstore();

    let _train_image = dataset.train_images.i(0);

    // TODO - merge method, so it returns both network and optimizer.
    //        Although an optimizer could be choosen independently.
    let net = network.create_network(&vs.root());
    let mut opt = network.create_optimizer(&vs)?;

    const TRAIN_BATCH_SIZE: i64 = 1;
    const TEST_BATCH_SIZE: i64 = 1;

    let epoch_steps = dataset.train_images.size()[0] / TRAIN_BATCH_SIZE;

    let mut epoch_pb = tqdm!(
        total = config.epochs,
        desc = "Training",
        animation = "fillup",
        unit = "epoch",
        force_refresh = true,
        position = 0,
        disable = true
    );

    for epoch in 0..config.epochs {
        let timer_epoch = Instant::now();

        opt.set_lr(network.learning_rate(epoch));
        for (bimages, blabels) in tqdm!(
            dataset
                .train_iter(TRAIN_BATCH_SIZE)
                // .shuffle()
                .to_device(vs.device())
                .take(10),
            total = epoch_steps as usize,
            desc = "Batches",
            unit = "batch",
            force_refresh = true,
            position = 1,
            disable = true
        ) {
            if config.dry_run {
                continue;
            }

            // let bimages = augmentation(&bimages, false, 4, 8);
            let loss = net
                .forward_t(&bimages, true)
                .cross_entropy_for_logits(&blabels);
            let loss = ensure_finite(&loss, "Loss");
            opt.backward_step(&loss);
            let loss: f64 = loss.into();
            println!("{loss:1.06}");
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

        epoch_pb.write(format!("EPOCH {epoch:>3}: {:>3.2}%", test_accuracy * 100.0));
        epoch_pb.update(1);
    }

    // let result: Vec<(u8, u8)> = dataset
    //     .test_iter(1)
    //     .shuffle()
    //     .to_device(vs.device())
    //     .map(|(image, label)| {
    //         let prediction = net
    //             .forward_t(&image, /*train=*/ false)
    //             .softmax(-1, tch::Kind::Float)
    //             .squeeze();
    //
    //         let (_, classes) = prediction.max_dim(-1, false);
    //         let label = u8::from(&label);
    //         let class = u8::from(&classes);
    //         (label, class)
    //     })
    //     .collect();
    //
    // let correct = result.iter().filter(|(a, b)| a == b).count() as f32;
    // let total = dataset.test_images.size()[0] as f32;
    //
    // println!(
    //     "Final validation: {:>3.2}%: {:?}",
    //     correct * 100.0 / total,
    //     result.into_iter().map(|(_, a)| a).collect::<Vec<_>>()
    // );

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

    println!(
        "Train set: ads {}/{:.02}, music {}/{:.02}",
        train_ads.0, train_ads.1, train_music.0, train_music.1
    );

    let test_classes = Vec::<i16>::from(&ds.test_labels);
    let test_ads = count(0, &test_classes);
    let test_music = count(1, &test_classes);

    println!(
        "Test set: ads {}/{:.02}, music {}/{:.02}",
        test_ads.0, test_ads.1, test_music.0, test_music.1
    );
}
