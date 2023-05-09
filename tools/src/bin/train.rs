use std::{
    collections::VecDeque,
    env::args,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

use classifier::Classifier;
use itertools::Itertools;
use ndarray::{concatenate, stack, Array1, Array3, Array4, ArrayBase, ArrayView, Axis};
use ndarray_shuffle::NdArrayShuffleInplaceExt;
use rand::{rngs::SmallRng, SeedableRng};

const W: usize = 150;
const H: usize = 39;

const TRAIN_CHUNK: usize = 16_384;
const VERIFICATION_CHUNK: usize = 256;

fn main() -> anyhow::Result<()> {
    println!(
        r#"
###############
###  TRAIN  ###
###############
        "#
    );
    println!("CHUNK {TRAIN_CHUNK}");
    train()?;

    println!(
        r#"
######################
###  VERIFICATION  ###
######################
        "#
    );
    println!("CHUNK {VERIFICATION_CHUNK}");
    verify()?;

    Ok(())
}

fn train() -> anyhow::Result<()> {
    let data_ads = get_data(&args().nth(1).expect("Ads train data"))?;
    let data_music = get_data(&args().nth(2).expect("Music train data"))?;

    let mut classifier = Classifier::new()?;

    let mut processed = 0;

    for chunk in &data_ads
        .zip(data_music)
        .chunks(TRAIN_CHUNK)
    {
        let (ads, music): (Vec<_>, Vec<_>) = chunk.unzip();

        println!("PROCESSING {}..{}", processed, processed + ads.len());
        processed += ads.len();

        let (data, labels) = prepare_data_and_labels(&ads, &music)?;
        classifier.train(&data, &labels, 3, 128)?;
        classifier.save("model")?;
    }

    Ok(())
}

fn verify() -> anyhow::Result<()> {
    let data_ads = get_data(&args().nth(1).expect("Ads train data"))?;
    let data_music = get_data(&args().nth(2).expect("Music train data"))?;

    let classifier = Classifier::from_file("model")?;
    for chunk in &data_ads
        .zip(data_music)
        .chunks(VERIFICATION_CHUNK)
    {
        let (ads, music): (Vec<_>, Vec<_>) = chunk.unzip();
        let (data, labels) = prepare_data_and_labels(&ads, &music)?;

        let predicted = classifier.predict(&data)?;
        let accuracy = classifier::verify(&predicted, &labels)?;
        println!("Accuracy: {accuracy:2.02}%");
    }

    Ok(())
}

fn prepare_data_and_labels(
    ads: &[Array3<f32>],
    music: &[Array3<f32>],
) -> anyhow::Result<(Array4<f32>, Array1<u32>)> {
    let (ads_data, ads_labels) = prepare(ads, 0)?;
    let (music_data, music_labels) = prepare(music, 1)?;

    let mut data = concatenate(
        Axis(0),
        &[ArrayView::from(&ads_data), ArrayView::from(&music_data)],
    )?;
    let mut labels = concatenate(
        Axis(0),
        &[ArrayView::from(&ads_labels), ArrayView::from(&music_labels)],
    )?;

    labels.shuffle_inplace_with(Axis(0), &mut SmallRng::seed_from_u64(0xFEEB))?;
    data.shuffle_inplace_with(Axis(0), &mut SmallRng::seed_from_u64(0xFEEB))?;

    Ok((data, labels))
}

fn prepare(data: &[Array3<f32>], label: u32) -> anyhow::Result<(Array4<f32>, Array1<u32>)> {
    let labels = Array1::<u32>::from_elem(data.len(), label);
    let data = stack(
        Axis(0),
        &data.iter().map(ArrayBase::from).collect::<Vec<_>>(),
    )?;

    assert!(data.iter().all(|x| x.is_finite()));

    Ok((data, labels))
}

fn get_data(path: &str) -> anyhow::Result<DataIterator> {
    println!("Loading {path}...");
    let mut data = BufReader::new(File::open(path)?);

    let mut count = 0;
    while let Ok(values) = bincode::deserialize_from::<_, Vec<f32>>(&mut data) {
        count += values.len() / (W * H);
    }
    data.seek(SeekFrom::Start(0))?;

    println!(
        "{} bytes, {} records",
        data.get_ref().metadata()?.len(),
        count
    );

    Ok(DataIterator::new(Box::new(data), count))
}

struct DataIterator {
    data: Box<dyn Read>,
    #[allow(dead_code)]
    size: usize,
    tail: VecDeque<Array3<f32>>,
}

impl DataIterator {
    fn new(data: Box<dyn Read>, size: usize) -> Self {
        Self {
            data,
            size,
            tail: VecDeque::new(),
        }
    }
}

impl Iterator for DataIterator {
    type Item = Array3<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.tail.is_empty() {
            return self.tail.pop_front();
        }

        while let Ok(block) = bincode::deserialize_from::<_, Vec<f32>>(&mut self.data.as_mut()) {
            self.tail = block
                .chunks_exact(W * H)
                .map(|chunk| Array3::from_shape_vec((W, H, 1), chunk.to_vec()))
                .collect::<Result<_, _>>()
                .unwrap();

            if self.tail.is_empty() {
                continue;
            }

            return self.tail.pop_front();
        }

        self.tail.clear();
        None
    }
}
