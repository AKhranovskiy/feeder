use tch::Tensor;

use crate::config::{CLASSIFICATION_SEGMENT_LENGTH, MFCC_N_COEFFS, SAMPLE_RATE};
use crate::{decode, mfcc};

pub(crate) async fn prepare_train_images(data: Vec<u8>) -> anyhow::Result<Tensor> {
    let data: Vec<f32> = decode::audio_to_pcm_s16le(data)
        .await?
        .into_iter()
        .map(f32::from)
        .collect();

    prepare_mfccs_tensor(&data).await
}

pub(crate) async fn prepare_classification_images(data: Vec<u8>) -> anyhow::Result<Tensor> {
    let data: Vec<f32> = decode::audio_to_pcm_s16le(data)
        .await?
        .into_iter()
        .map(f32::from)
        .collect();

    let segments = (data.len() - CLASSIFICATION_SEGMENT_LENGTH) / SAMPLE_RATE;
    let mut output = Vec::<Tensor>::with_capacity(segments);

    // Drop last uncomplete segment.
    for i in 0..segments {
        let start = SAMPLE_RATE * i;
        let segment = &data[start..start + CLASSIFICATION_SEGMENT_LENGTH];
        output.push(prepare_mfccs_tensor(segment).await?);
    }

    Ok(Tensor::concatenate(&output, 0))
}

async fn prepare_mfccs_tensor(data: &[f32]) -> anyhow::Result<Tensor> {
    let mut mfccs = mfcc::calculate(data).await?;

    let chunk_size = MFCC_N_COEFFS * 150;
    let num_chunks = mfccs.len() / chunk_size;
    let aligned_size = num_chunks * chunk_size;

    if aligned_size != mfccs.len() {
        let mut aligned_mfccs = Vec::with_capacity(mfccs.len() * 2 - aligned_size);
        aligned_mfccs.extend_from_within(mfccs.len() - aligned_size..);
        mfccs = aligned_mfccs;
    }

    anyhow::ensure!(mfccs.len() % chunk_size == 0, "Misaligned MFCC data");

    Ok(Tensor::try_from(mfccs)?
        .reshape(&[-1, 1, 150, MFCC_N_COEFFS as i64])
        .transpose(2, 3)
        .to_device(tch::Device::cuda_if_available()))
}

pub(crate) async fn prepare_batch_train_dataset(
    mfccs: Vec<Vec<f32>>,
) -> anyhow::Result<tch::vision::dataset::Dataset> {
    // Normalize

    let mfccs = mfccs.into_iter().map(normalize).collect::<Vec<_>>();

    // Align data
    const CHUNK_SIZE: usize = MFCC_N_COEFFS * 150;

    let max_length: usize = mfccs.iter().map(Vec::len).max().unwrap_or_default();
    let aligned_length =
        ((max_length as f32 / CHUNK_SIZE as f32).ceil() * CHUNK_SIZE as f32).trunc() as usize;

    println!("Prepare batch dataset: labels={}, max_length={max_length}, aligned_length={aligned_length}", mfccs.len());

    let images: Vec<Tensor> = mfccs
        .into_iter()
        .map(|mut v| {
            v.extend_from_within(2 * v.len() - aligned_length..);
            v
        })
        .map(|v| Tensor::try_from(v).map_err(|e| e.into()))
        .collect::<anyhow::Result<Vec<Tensor>>>()?
        .into_iter()
        .map(|t| {
            t.reshape(&[-1, 1, 150, MFCC_N_COEFFS as i64])
                .transpose(2, 3)
                .to_device(tch::Device::cuda_if_available())
        })
        .collect::<Vec<Tensor>>();

    for (index, t) in images.iter().enumerate() {
        println!("Data label={index}, size={:?}", t.size());
    }

    Ok(prepare_dataset(images, 0.1))
}

fn label_tensor((value, tensor): (usize, &Tensor)) -> Tensor {
    Tensor::full(
        &tensor.size()[0..1],
        value as i64,
        (tch::Kind::Int64, tch::Device::cuda_if_available()),
    )

    // let mut label = vec![0f32; 2];
    // label[value] = 1.0;
    // let size = tensor.size()[0];
    // Tensor::try_from(&label)
    //     .expect("Label tensor")
    //     .repeat(&[size, 1])
    //     .to_device(tch::Device::cuda_if_available())
}

fn prepare_dataset(images: Vec<Tensor>, test_set_size: f64) -> tch::vision::dataset::Dataset {
    assert!(
        test_set_size > 0.0 && test_set_size <= 1.0,
        "Test set size must be in range (0.0, 1.0), given: {test_set_size}."
    );

    let labels = images.len() as i64;

    let (train_set, test_set): (Vec<Tensor>, Vec<Tensor>) = images
        .into_iter()
        .map(|t| split_tensor(t, test_set_size))
        .unzip();

    let train_images = Tensor::concat(&train_set, 0);
    let train_labels = Tensor::concat(
        &train_set
            .iter()
            .enumerate()
            .map(label_tensor)
            .collect::<Vec<Tensor>>(),
        0,
    );

    assert_eq!(
        train_images.size()[0],
        train_labels.size()[0],
        "Train images and Labels mismatch"
    );

    let test_images = Tensor::concat(&test_set, 0);
    let test_labels = Tensor::concat(
        &test_set
            .iter()
            .enumerate()
            .map(label_tensor)
            .collect::<Vec<Tensor>>(),
        0,
    );

    println!("Train images set size: {:?}", train_images.size());
    println!("Test images set size: {:?}", test_images.size());
    println!("Train labels set size: {:?}", train_labels.size());
    println!("Test labels set size: {:?}", test_labels.size());

    tch::vision::dataset::Dataset {
        train_images,
        train_labels,
        test_images,
        test_labels,
        labels,
    }
}

fn split_tensor(t: Tensor, test_set_size: f64) -> (Tensor, Tensor) {
    let size = t.size()[0];
    assert!(size > 0, "Empty tensor");

    let pivot = size as f64 * (1f64 - test_set_size);
    let pivot = pivot.trunc() as i64;
    println!("Test set size: {test_set_size}. Pivot point: {pivot}");

    split_data(t, pivot)
}

fn split_data(data: Tensor, pivot: i64) -> (Tensor, Tensor) {
    let t = data.split_with_sizes(&[pivot, data.size()[0] - pivot], 0);
    (t[0].copy(), t[1].copy())
}

fn normalize(data: Vec<f32>) -> Vec<f32> {
    data
    // let (min, max) = data.iter().fold((f32::MAX, f32::MIN), |acc, &value| {
    //     (acc.0.min(value), acc.1.max(value))
    // });
    //
    // let range = max - min;
    //
    // if range > 1e-8 {
    //     data.into_iter().map(|v| (v - min) / range).collect()
    // } else {
    //     data.into_iter().map(|v| v - min).collect()
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        assert_eq!(
            vec![0., 1. / 6., 2. / 6., 3. / 6., 4. / 6., 5. / 6., 1.],
            normalize(vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2., 3.])
        );
    }
}
