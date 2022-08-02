use std::path::Path;
use std::time::Instant;

use anyhow::anyhow;

use model::ContentKind;
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use tch::nn::{ModuleT, OptimizerConfig};
use tch::vision::dataset::Dataset;
use tch::{nn, Device, IndexOp, Tensor};

use crate::network::cnn_projectpro::cnn_projectpro;

mod network;

const LIMIT: usize = 1000;

fn read_bin(path: &Path) -> anyhow::Result<Tensor> {
    std::fs::read(&path)
        .map_err(|e| anyhow!("Error reading bin {}: {:#?}", path.display(), e))
        .and_then(|bin| {
            bincode::deserialize::<mfcc::MFCCs>(&bin)
                .map_err(|e| anyhow!("Failed to load bin {}: {}", path.display(), e))
        })
        .and_then(|mfccs| {
            Tensor::try_from(mfccs)
                .map(|t| t.transpose(0, 1).reshape(&[1, 39, 171]))
                .map_err(|e| e.into())
        })
}

fn random_indices(max: usize, n: usize) -> Vec<usize> {
    let mut rng = thread_rng();
    let range = Uniform::new(0, max);
    (&mut rng).sample_iter(range).take(n).collect()
}

fn path_to_label(p: &Path) -> anyhow::Result<Tensor> {
    p.file_name()
        .ok_or_else(|| anyhow!("Failed to get filename"))
        .and_then(|p| p.to_str().ok_or_else(|| anyhow!("Failed to get string")))
        .and_then(|name| {
            name.split_once('|')
                .ok_or_else(|| anyhow!("Failed to split"))
        })
        .and_then(|(kind, _)| match ContentKind::try_from(kind) {
            Ok(ContentKind::Advertisement) => Ok(0),
            Ok(ContentKind::Music) => Ok(1),
            Ok(ContentKind::Talk) => Ok(2),
            _ => Err(anyhow!("Invalid kind: {}", kind)),
        })
        .map(|kind| Tensor::of_slice(&[kind as i64]).to_device(Device::cuda_if_available()))
}

fn load_data() -> anyhow::Result<(Tensor, Tensor)> {
    let files = std::fs::read_dir("bins")?
        .filter_map(|e| {
            if let Ok(entry) = e && entry.path().is_file() { Some(entry.path()) } else { None}
        })
        .collect::<Vec<_>>();

    println!("Total files: {}", files.len());

    let indices = random_indices(files.len(), LIMIT);

    let files = indices
        .par_iter()
        .map(|idx| files[*idx].clone())
        .collect::<Vec<_>>();

    let images = files
        .par_iter()
        .map(|p| read_bin(p))
        .collect::<Result<Vec<_>, _>>()?;

    let images = Tensor::stack(&images, 0);

    let labels = files
        .par_iter()
        .map(|p| path_to_label(p))
        .collect::<Result<Vec<_>, _>>()?;

    let labels = Tensor::concat(&labels, 0);

    Ok((images, labels))
}

fn split_data(data: Tensor, pivot: i64) -> (Tensor, Tensor) {
    let t = data.split_with_sizes(&[pivot, data.size()[0] - pivot], 0);
    (t[0].copy(), t[1].copy())
}

fn prepare_dataset(images: Tensor, labels: Tensor, test_set_size: f64) -> Dataset {
    let image_set_size = images.size()[0];
    let label_set_size = labels.size()[0];

    assert!(image_set_size > 0, "Image set may not be empty.");
    assert_eq!(
        image_set_size, label_set_size,
        "Image and Label sets must have equal size. {image_set_size} != {label_set_size}."
    );
    assert!(
        test_set_size > 0.0 && test_set_size <= 1.0,
        "Test set size must be in range (0.0, 1.0), given: {test_set_size}."
    );

    let pivot = image_set_size as f64 * (1f64 - test_set_size);
    let pivot = pivot.trunc() as i64;
    println!("Test set size: {test_set_size}. Pivot point: {pivot}");

    let (train_images, test_images) = split_data(images, pivot);
    let (train_labels, test_labels) = split_data(labels, pivot);

    println!("Train images set size: {:?}", train_images.size());
    println!("Test images set size: {:?}", test_images.size());
    println!("Train labels set size: {:?}", train_labels.size());
    println!("Test labels set size: {:?}", test_labels.size());

    Dataset {
        train_images,
        train_labels,
        test_images,
        test_labels,
        labels: 3,
    }
}

fn get_varstore() -> (nn::VarStore, bool) {
    let mut varstore = nn::VarStore::new(Device::cuda_if_available());
    let loaded = varstore.load("model.tch").is_ok();
    (varstore, loaded)
}

fn main() -> anyhow::Result<()> {
    let now = Instant::now();

    let (images, labels) = load_data()?;

    println!("Loaded {:?} samples", labels.size());

    println!("elapsed {:0.2}s", now.elapsed().as_secs_f32());

    let dataset = prepare_dataset(images, labels, 0.25);

    let (vs, loaded) = get_varstore();
    let net = cnn_projectpro(&vs.root());

    let timer_total = Instant::now();

    if !loaded {
        let mut opt = nn::Sgd {
            momentum: 0.9,
            dampening: 0.,
            wd: 5e-4,
            nesterov: true,
        }
        .build(&vs, 0.)?;

        for epoch in 1..150 {
            let timer_epoch = Instant::now();

            opt.set_lr(learning_rate(epoch));
            for (bimages, blabels) in dataset.train_iter(64).shuffle().to_device(vs.device()) {
                let bimages = tch::vision::dataset::augmentation(&bimages, false, 4, 8);
                let loss = net
                    .forward_t(&bimages, true)
                    .cross_entropy_for_logits(&blabels);
                opt.backward_step(&loss);
            }
            let test_accuracy = net.batch_accuracy_for_logits(
                &dataset.test_images,
                &dataset.test_labels,
                vs.device(),
                512,
            );
            println!(
                "epoch: {:4} test acc: {:5.2}%, elapsed: {:.02}s",
                epoch,
                100. * test_accuracy,
                timer_epoch.elapsed().as_secs_f32()
            );
        }
    } else {
        println!("Prediction");

        println!(" ADS | MUS | TLK | ORG ");
        println!("-----------------------");

        for (image, label) in dataset.test_iter(10).take(1) {
            let output = net
                .forward_t(&image, /*train=*/ false)
                .softmax(-1, tch::Kind::Float);
            // output.print();

            let values = Vec::<f64>::from(output);
            let labels = Vec::<i64>::from(label);

            for (vals, lbl) in values.chunks_exact(3).zip(labels) {
                println!(
                    "{:^5}|{:^5}|{:^5}|{}",
                    (vals[0] * 100f64).round() as i64,
                    (vals[1] * 100f64).round() as i64,
                    (vals[2] * 100f64).round() as i64,
                    match lbl {
                        0 => "ADS",
                        1 => "MUS",
                        2 => "TLK",
                        _ => unreachable!(),
                    }
                );
            }
        }
    }

    println!(
        "total elapsed: {:.02}s",
        timer_total.elapsed().as_secs_f32()
    );

    vs.save(Path::new("model.tch"))?;

    Ok(())
}

fn learning_rate(epoch: i64) -> f64 {
    if epoch < 50 {
        0.1
    } else if epoch < 100 {
        0.01
    } else {
        0.001
    }
}
