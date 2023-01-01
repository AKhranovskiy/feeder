use std::path::Path;

use ndarray::{s, Axis};

use classifier::{verify, Classifier};
use ndarray_shuffle::NdArrayShuffleInplaceExt;
use rand::rngs::SmallRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    println!("Loading data...");
    let (data, labels) = prepare_data("./data.pickle")?;

    println!("Loaded {} images", labels.len());

    println!("data={:?} labels={:?}", data.shape(), labels.shape());

    let mut classifier = Classifier::new()?;
    classifier.train(&data, &labels)?;
    classifier.save("model")?;

    let owned_results = data
        .axis_chunks_iter(Axis(0), 413)
        .map(|chunk| {
            classifier
                .predict(&chunk.to_owned())
                .expect("Python function returned result")
        })
        .collect::<Vec<_>>();

    let predicted = ndarray::concatenate(
        Axis(0),
        owned_results
            .iter()
            .map(|v| v.view())
            .collect::<Vec<_>>()
            .as_ref(),
    )?;

    let accuracy = verify(&predicted, &labels)?;

    println!("Accuracy: {accuracy:2.02}%");

    Ok(())
}

fn prepare_data<P>(source: P) -> anyhow::Result<(ndarray::Array4<f64>, ndarray::Array1<u32>)>
where
    P: AsRef<Path>,
{
    let f = std::fs::File::open(source)?;
    let reader = std::io::BufReader::new(f);

    let data: ndarray::Array3<f64> = serde_pickle::from_reader(reader, Default::default())?;
    assert_eq!(3, data.shape()[0]);
    assert_eq!(39, data.shape()[2]);

    let min_len = data.shape()[1];
    let min_len = min_len - (min_len % 150);
    assert_eq!(0, min_len % 150);

    let data = data.slice(s![0..2, 0..min_len, ..]).into_owned();

    let number_of_images = (2 * min_len) / 150;
    let mut data = data.into_shape((number_of_images, 150, 39, 1))?;

    let mut labels = ndarray::concatenate![
        ndarray::Axis(0),
        ndarray::Array1::from_elem((number_of_images / 2,), 0),
        ndarray::Array1::from_elem((number_of_images / 2,), 1),
        // ndarray::Array1::from_elem((number_of_images / 3,), 2)
    ];

    data.shuffle_inplace_with(Axis(0), &mut SmallRng::seed_from_u64(0xFEEB))?;
    labels.shuffle_inplace_with(Axis(0), &mut SmallRng::seed_from_u64(0xFEEB))?;

    Ok((data, labels))
}
