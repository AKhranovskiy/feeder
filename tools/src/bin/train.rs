use std::env::args;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use ndarray::{concatenate, Array1, Array4, ArrayBase, Axis};

use classifier::{verify, Classifier};

const BLOCK: usize = 150 * 39;

fn main() -> anyhow::Result<()> {
    let bindir = Path::new(&args().nth(1).expect("Path to bin dir")).to_owned();

    println!("Loading data from {}", bindir.display());
    let (data, labels) = prepare_data(bindir)?;

    println!("Training...");
    let mut classifier = Classifier::new()?;
    classifier.train(&data, &labels)?;
    classifier.save("model")?;

    println!("Verification...");
    let owned_results = data
        .axis_chunks_iter(Axis(0), 413)
        .map(|chunk| {
            classifier
                .predict(&chunk.to_owned())
                .expect("Python function returned result")
        })
        .collect::<Vec<_>>();

    let predicted = concatenate(
        Axis(0),
        owned_results
            .iter()
            .map(ndarray::ArrayBase::view)
            .collect::<Vec<_>>()
            .as_ref(),
    )?;

    let accuracy = verify(&predicted, &labels)?;

    println!("Accuracy: {accuracy:2.02}%");

    Ok(())
}

fn prepare_data<P>(bindir: P) -> anyhow::Result<(ndarray::Array4<f32>, ndarray::Array1<u32>)>
where
    P: AsRef<Path>,
{
    let data: Vec<Vec<f32>> = [
        bindir.as_ref().join("ads.bin"),
        bindir.as_ref().join("music.bin"),
    ]
    .iter()
    .map(|source| bincode::deserialize_from(BufReader::new(File::open(source)?)))
    .collect::<Result<_, _>>()?;

    let data = data
        .into_iter()
        .map(|mut v| {
            let len = v.len();
            let len = len - (len % BLOCK);
            assert_eq!(0, len % BLOCK);
            println!("{:?}->{:?}", v.len(), len);
            v.truncate(len);
            Array4::from_shape_vec((len / BLOCK, BLOCK / 39, 39, 1), v)
        })
        .collect::<Result<Vec<_>, _>>()?;

    println!(
        "Loaded {:?} images",
        data.iter().map(|x| x.shape()[0]).collect::<Vec<_>>()
    );

    let labels = data
        .iter()
        .enumerate()
        .map(|x| Array1::<u32>::from_elem(x.1.shape()[0], x.0 as u32))
        .fold(Array1::from_elem(0, 0), |mut acc, x| {
            acc.append(Axis(0), x.view()).unwrap();
            acc
        });
    println!("labels={:?}", labels.shape());

    let data = concatenate(
        Axis(0),
        data.iter()
            .map(ArrayBase::view)
            .collect::<Vec<_>>()
            .as_slice(),
    )?;
    println!("data={:?}", data.shape());

    assert!(data.iter().all(|x| x.is_finite()));
    // println!("Sort labels...");
    // labels.shuffle_inplace_with(Axis(0), &mut SmallRng::seed_from_u64(0xFEEB))?;
    // println!("Sort data...");
    // data.shuffle_inplace_with(Axis(0), &mut SmallRng::seed_from_u64(0xFEEB))?;

    println!("Done");

    Ok((data, labels))
}
