use std::{
    env::args,
    fs::File,
    io::{BufReader, Read},
};

use classifier::{Classifier, Data, Labels};
use itertools::Itertools;
use ndarray::{concatenate, stack, Array1, ArrayBase, ArrayView, Axis};
use ndarray_shuffle::NdArrayShuffleInplaceExt;
use rand::{rngs::SmallRng, SeedableRng};

const TRAIN_CHUNK: usize = 128;
const VERIFICATION_CHUNK: usize = 64;

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

    for chunk in &data_ads.zip(data_music).chunks(TRAIN_CHUNK) {
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
    for chunk in &data_ads.zip(data_music).chunks(VERIFICATION_CHUNK) {
        let (ads, music): (Vec<_>, Vec<_>) = chunk.unzip();
        let (data, labels) = prepare_data_and_labels(&ads, &music)?;

        let predicted = classifier.predict(&data)?;
        let accuracy = classifier::verify(&predicted, &labels)?;
        println!("Accuracy: {accuracy:2.02}%");
    }

    Ok(())
}

fn prepare_data_and_labels(
    ads: &[Array1<i16>],
    music: &[Array1<i16>],
) -> anyhow::Result<(Data, Labels)> {
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

fn prepare(data: &[Array1<i16>], label: u32) -> anyhow::Result<(Data, Labels)> {
    let labels = Labels::from_elem(data.len(), label);
    let data = stack(
        Axis(0),
        &data.iter().map(ArrayBase::from).collect::<Vec<_>>(),
    )?;

    Ok((data, labels))
}

fn get_data(path: &str) -> anyhow::Result<DataIterator> {
    println!("Loading {path}...");
    let data = BufReader::new(File::open(path)?);
    Ok(DataIterator::new(Box::new(data)))
}

struct DataIterator {
    data: Box<dyn Read>,
}

impl DataIterator {
    fn new(data: Box<dyn Read>) -> Self {
        Self { data }
    }
}

impl Iterator for DataIterator {
    type Item = Array1<i16>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(block) = bincode::deserialize_from::<_, Vec<i16>>(&mut self.data.as_mut()) {
            Some(block.into())
        } else {
            None
        }
    }
}
