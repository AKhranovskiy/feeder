use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use flate2::read::DeflateDecoder;
use tch::Tensor;

pub const NN_IMAGE_W: usize = 200;
pub const NN_IMAGE_H: usize = 39;

pub fn load_data(path: &Path) -> anyhow::Result<Vec<Tensor>> {
    let buf = {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut decoder = DeflateDecoder::new(reader);
        let mut buf = vec![];
        decoder.read_to_end(&mut buf)?;
        buf
    };

    let data: Vec<Vec<f32>> = bincode::deserialize_from(buf.as_slice())?;

    anyhow::ensure!(
        data.len() == 2,
        "Bin must contain 2 arrays, found {}",
        data.len()
    );

    anyhow::ensure!(
        data.iter().flat_map(|v| v.iter()).all(|f| f.is_finite()),
        "Data contains NaN or infinities"
    );

    Ok(data
        .into_iter()
        .map(linear_to_tensor_t::<{ NN_IMAGE_W }, { NN_IMAGE_H }>)
        .collect())
}

fn full_tensor((value, tensor): (usize, &Tensor)) -> Tensor {
    let size = tensor.size()[0];
    Tensor::full(&[size], value as i64, (tch::Kind::Int64, tch::Device::Cpu))
}

fn split_data(data: tch::Tensor, pivot: i64) -> (tch::Tensor, tch::Tensor) {
    let t = data.split_with_sizes(&[pivot, data.size()[0] - pivot], 0);
    (t[0].copy(), t[1].copy())
}

pub fn prepare_dataset(
    images: Vec<tch::Tensor>,
    test_set_size: f64,
) -> tch::vision::dataset::Dataset {
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
            .map(full_tensor)
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
            .map(full_tensor)
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

/// Convert a linear[X * W * H] array to a tensor of [X, 1, H, W].
///
/// Example:
/// [1,2,3,4,5,6,7,8,9,10,11,12] -> convert::<3,2>() ->
///   1 3 5   7  9 11
///   2 4 6   8 10 12
fn linear_to_tensor_t<const W: usize, const H: usize>(input: Vec<f32>) -> tch::Tensor {
    let chunk = W * H;
    let len = (input.len() / chunk) * chunk;
    tch::Tensor::from(&input[..len])
        .reshape(&[(len / chunk) as i64, 1, W as i64, H as i64])
        .transpose(2, 3)
}

#[cfg(test)]
mod tests {
    use super::linear_to_tensor_t;

    #[test]
    fn test_convert() {
        let data: Vec<f32> = (0u8..45).map(f32::from).collect();
        let t = linear_to_tensor_t::<5, 3>(data);
        assert_eq!(t.size(), [3, 1, 3, 5]);
    }
}
