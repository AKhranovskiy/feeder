use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use anyhow::anyhow;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::utils::random_bucket_indices;

pub const IMAGE_TENSOR_SHAPE: [i64; 3] = [1, 39, 171];

pub fn load_data(
    path: &Path,
    samples: &Option<usize>,
) -> anyhow::Result<(tch::Tensor, tch::Tensor)> {
    let mut files = std::fs::read_dir(path)?
        .filter_map(|e| {
            if let Ok(entry) = e {
                if entry.path().is_file() {
                    Some(entry.path())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    println!("Total files: {}", files.len());

    files.sort_unstable();

    let file_groups = files
        .as_slice()
        .group_by(|a, b| match (a.file_name(), b.file_name()) {
            (Some(a), Some(b)) => a.as_bytes()[0..4] == b.as_bytes()[0..4],
            _ => false,
        })
        .collect::<Vec<_>>();

    #[allow(clippy::string_from_utf8_as_bytes)]
    for &fg in &file_groups {
        println!(
            "Label {}, count: {}",
            std::str::from_utf8(&fg[0].file_name().unwrap().as_bytes()[0..4]).unwrap(),
            fg.len()
        );
    }

    // Indices must be shuffled so train/test split takes all classes.
    // TODO - Separate train/test files before selecting to guarantee presence of all classes in both sets.
    let indices = random_bucket_indices(
        &file_groups.iter().map(|x| x.len()).collect::<Vec<_>>(),
        samples.unwrap_or(files.len()),
    );

    let files = indices
        .par_iter()
        .map(|idx| files[*idx].clone())
        .collect::<Vec<_>>();

    let images = files
        .par_iter()
        .map(|p| read_bin(p))
        .collect::<Result<Vec<_>, _>>()?;

    let images = tch::Tensor::stack(&images, 0);

    let labels = files
        .par_iter()
        .map(|p| path_to_label(p))
        .collect::<Result<Vec<_>, _>>()?;

    let labels = tch::Tensor::concat(&labels, 0);

    Ok((images, labels))
}

fn read_bin(path: &Path) -> anyhow::Result<tch::Tensor> {
    std::fs::read(path)
        .map_err(|e| anyhow!("Error reading bin {}: {:#?}", path.display(), e))
        .and_then(|bin| {
            bincode::deserialize::<mfcc::MFCCs>(&bin)
                .map_err(|e| anyhow!("Failed to load bin {}: {}", path.display(), e))
        })
        .and_then(|mfccs| {
            tch::Tensor::try_from(mfccs)
                .map(|t| t.transpose(0, 1).reshape(&IMAGE_TENSOR_SHAPE))
                .map_err(|e| e.into())
        })
}

fn path_to_label(p: &Path) -> anyhow::Result<tch::Tensor> {
    p.file_name()
        .ok_or_else(|| anyhow!("Failed to get filename"))
        .and_then(|p| p.to_str().ok_or_else(|| anyhow!("Failed to get string")))
        .and_then(|name| {
            name.split_once('|')
                .ok_or_else(|| anyhow!("Failed to split"))
        })
        .and_then(|(kind, _)| match model::ContentKind::try_from(kind) {
            Ok(model::ContentKind::Advertisement) => Ok(0),
            Ok(model::ContentKind::Music) => Ok(1),
            Ok(model::ContentKind::Talk) => Ok(2),
            _ => Err(anyhow!("Invalid kind: {}", kind)),
        })
        .map(|kind| {
            tch::Tensor::of_slice(&[kind as i64]).to_device(tch::Device::cuda_if_available())
        })
}

fn split_data(data: tch::Tensor, pivot: i64) -> (tch::Tensor, tch::Tensor) {
    let t = data.split_with_sizes(&[pivot, data.size()[0] - pivot], 0);
    (t[0].copy(), t[1].copy())
}

pub fn prepare_dataset(
    images: tch::Tensor,
    labels: tch::Tensor,
    test_set_size: f64,
) -> tch::vision::dataset::Dataset {
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
    // println!("Test set size: {test_set_size}. Pivot point: {pivot}");

    let (train_images, test_images) = split_data(images, pivot);
    let (train_labels, test_labels) = split_data(labels, pivot);

    // println!("Train images set size: {:?}", train_images.size());
    // println!("Test images set size: {:?}", test_images.size());
    // println!("Train labels set size: {:?}", train_labels.size());
    // println!("Test labels set size: {:?}", test_labels.size());

    println!(
        "Selected: {image_set_size}, train={}, test={}",
        train_images.size()[0],
        test_images.size()[0]
    );

    tch::vision::dataset::Dataset {
        train_images,
        train_labels,
        test_images,
        test_labels,
        labels: 3,
    }
}
