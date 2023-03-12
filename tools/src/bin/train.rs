use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use ndarray::{s, Array1, Array2, Axis};

use classifier::{verify, Classifier};

const BLOCK: usize = 150;

fn main() -> anyhow::Result<()> {
    println!("Loading data...");
    let (data, labels) = prepare_data(&["./ads.bin", "./music.bin"])?;

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

    let predicted = ndarray::concatenate(
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

fn prepare_data<P>(sources: &[P]) -> anyhow::Result<(ndarray::Array4<f64>, ndarray::Array1<u32>)>
where
    P: AsRef<Path>,
{
    let data: Vec<Array2<f64>> = sources
        .iter()
        .map(|source| bincode::deserialize_from(BufReader::new(File::open(source)?)))
        .collect::<Result<_, _>>()?;

    let views = data
        .iter()
        .map(|x| {
            let len = x.shape()[0];
            let len = len - (len % BLOCK);
            assert_eq!(0, len % BLOCK);
            println!("{:?}->{:?}", x.shape(), (len, x.shape()[1]));
            x.slice(s![0..len, ..])
                .into_shape((len / BLOCK, BLOCK, 39, 1))
        })
        .collect::<Result<Vec<_>, _>>()?;

    println!(
        "Loaded {:?} images",
        views.iter().map(|x| x.shape()[0]).collect::<Vec<_>>()
    );

    let labels = views
        .iter()
        .enumerate()
        .map(|x| Array1::<u32>::from_elem(x.1.shape()[0], x.0 as u32))
        .fold(Array1::from_elem(0, 0), |mut acc, x| {
            acc.append(Axis(0), x.view()).unwrap();
            acc
        });
    println!("labels={:?}", labels.shape());

    let data = ndarray::concatenate(Axis(0), views.as_slice())?;
    println!("data={:?}", data.shape());

    assert!(data.iter().all(|x| x.is_finite()));
    // println!("Sort labels...");
    // labels.shuffle_inplace_with(Axis(0), &mut SmallRng::seed_from_u64(0xFEEB))?;
    // println!("Sort data...");
    // data.shuffle_inplace_with(Axis(0), &mut SmallRng::seed_from_u64(0xFEEB))?;

    println!("Done");

    Ok((data, labels))
}
