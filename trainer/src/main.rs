use std::time::Instant;

use anyhow::{bail, Context};
use automl::settings::Algorithm;
use bytes::Buf;
use mfcc::{calculate_mel_coefficients_with_deltas, ffmpeg_decode, RawAudioData};
use model::{ContentKind, MetadataWithAudio};
use ndarray::s;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use reqwest::header::ACCEPT;
use reqwest::{Client, StatusCode, Url};

const ENDPOINT: &str = "http://localhost:8000";
const MSGPACK_MIME: &str = "application/msgpack";
const LIMIT: usize = 1000;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let now = Instant::now();

    let (ads, music, talk) = tokio::try_join!(
        fetch_data(0, LIMIT, ContentKind::Advertisement),
        fetch_data(0, LIMIT, ContentKind::Music),
        fetch_data(0, LIMIT, ContentKind::Talk),
    )?;

    println!(
        "Data fetched, ads={}, music={}, talk={}, elapsed={:0.2}s",
        ads.len(),
        music.len(),
        talk.len(),
        now.elapsed().as_secs_f32()
    );

    let (mut mel_ads, mut mel_music, mut mel_talk) =
        tokio::try_join!(process(&ads), process(&music), process(&talk))?;

    // for item in &mel_ads {
    //     for (j, chunk) in item.iter().enumerate() {
    //         mfcc::plot(chunk, &format!("ads-{j}"));
    //     }
    // }
    // for item in &mel_music {
    //     for (j, chunk) in item.iter().enumerate() {
    //         mfcc::plot(chunk, &format!("music-{j}"));
    //     }
    // }
    // for item in &mel_talk {
    //     for (j, chunk) in item.iter().enumerate() {
    //         mfcc::plot(chunk, &format!("talk-{j}"));
    //     }
    // }

    println!(
        "Dataset: ads={}, music={}, talk={}, elapsed={:0.2}s",
        mel_ads.len(),
        mel_music.len(),
        mel_talk.len(),
        now.elapsed().as_secs_f32()
    );

    let dataset_len = mel_ads.len() + mel_music.len() + mel_talk.len();
    let mut classes = Vec::with_capacity(dataset_len);
    classes.append(&mut vec![100f32; mel_ads.len()]);
    classes.append(&mut vec![0f32; mel_music.len()]);
    classes.append(&mut vec![-100f32; mel_talk.len()]);

    let mut dataset = Vec::with_capacity(dataset_len);
    dataset.append(&mut mel_ads);
    dataset.append(&mut mel_music);
    dataset.append(&mut mel_talk);

    println!(
        "Data prepared, elapsed={:0.2}s",
        now.elapsed().as_secs_f32()
    );

    let settings = automl::Settings::default_classification()
        .skip(Algorithm::CategoricalNaiveBayes)
        .shuffle_data(true)
        .verbose(true);
    let mut classifier = automl::SupervisedModel::new((dataset, classes), settings);
    classifier.train();
    println!("{classifier}");

    println!("elapsed {:0.2}s", now.elapsed().as_secs_f32());
    Ok(())
}

async fn process(items: &Vec<MetadataWithAudio>) -> anyhow::Result<Vec<Vec<f32>>> {
    items
        .par_iter()
        .map(|item| &item.content)
        .cloned()
        .map(ffmpeg_decode)
        .map(|audio| audio.and_then(calculate_mel))
        .collect::<Result<Vec<_>, _>>()
        .map(|vv| {
            vv.into_iter()
                .flat_map(IntoIterator::into_iter)
                .map(|a| a.into_raw_vec())
                .collect::<Vec<_>>()
        })
}

async fn fetch_data(
    skip: usize,
    limit: usize,
    kind: ContentKind,
) -> anyhow::Result<Vec<MetadataWithAudio>> {
    let mut url = Url::parse(ENDPOINT)?.join("/api/v1/segments/msgpack")?;

    url.set_query(Some(&format!(
        "skip={skip}&limit={limit}&kind={}",
        kind.to_string(),
    )));

    let response = Client::new()
        .get(url)
        .header(ACCEPT, MSGPACK_MIME)
        .send()
        .await
        .context("Sending reqwest")?;

    if let StatusCode::OK | StatusCode::CREATED = response.status() {
        rmp_serde::from_read(response.bytes().await?.reader()).map_err(|e| e.into())
    } else {
        bail!("{} {}", response.status(), response.text().await?);
    }
}

fn calculate_mel(data: RawAudioData) -> anyhow::Result<Vec<mfcc::MFCCs>> {
    let window_size = mfcc::SAMPLE_RATE as usize * 4; // 4 secs
    let window_step = mfcc::SAMPLE_RATE as usize; // 1 sec
    let steps = ((data.len() - window_size) as f32 / window_step as f32).round() as usize + 1;

    let mut mels = Vec::new();
    for step in 0..steps {
        let window_start = step * window_step;
        let window_end = (window_start + window_size).min(data.len());
        let data = data.slice(s![window_start..window_end]).to_owned();
        mels.push(calculate_mel_coefficients_with_deltas(&data)?);
    }
    Ok(mels)
}
