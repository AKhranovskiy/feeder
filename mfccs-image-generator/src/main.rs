use std::fs::File;
use std::io::Write;
use std::time::Instant;

use anyhow::{bail, Context};
use bytes::Buf;
use mfcc::{calculate_mel_coefficients_with_deltas, ffmpeg_decode, RawAudioData};
use model::{ContentKind, MetadataWithAudio};
use ndarray::s;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use reqwest::{blocking::Client, StatusCode, Url};

const ENDPOINT: &str = "http://localhost:3456";
const MSGPACK_MIME: &str = "application/msgpack";
const STEP_SIZE: usize = 100;

fn main() -> anyhow::Result<()> {
    let total_timer = Instant::now();

    for skip in (0..).step_by(STEP_SIZE) {
        let step_timer = Instant::now();

        // 1. Fetch audio data.
        println!("Fetching data {skip}..");
        let op_timer = Instant::now();
        let data = fetch_data(skip, STEP_SIZE)?;
        println!(
            "Fetched {}, elapsed {}ms",
            data.len(),
            op_timer.elapsed().as_millis()
        );

        if data.is_empty() {
            break;
        }

        // 2. Filter out unknown kind.
        println!("Filtering data..");
        let op_timer = Instant::now();
        let data = data
            .iter()
            .filter(|d| d.kind != ContentKind::Unknown)
            .collect::<Vec<_>>();
        println!(
            "Filtered {}, elapsed {}ms",
            data.len(),
            op_timer.elapsed().as_millis()
        );

        // 3. Decode audio.
        println!("Decoding audio..");
        let op_timer = Instant::now();
        let data2 = data
            .par_iter()
            .map(|d| ffmpeg_decode(&d.content))
            .collect::<Result<Vec<_>, _>>()?;
        println!("Decoded, elapsed {}ms", op_timer.elapsed().as_millis());

        // 4. Calculate MFCCs;
        println!("Calculating MFCCs..");
        let op_timer = Instant::now();
        let data2 = data2
            .into_par_iter()
            .map(|d| calculate_mfccs(&d))
            .collect::<Result<Vec<_>, _>>()?;
        println!("Calculated, elapsed {}ms", op_timer.elapsed().as_millis());

        // 5. Zip audio title with mfccs.
        println!("Flattening MFCCs..");
        let op_timer = Instant::now();
        let data4 = data
            .iter()
            .map(|d| d.title.clone())
            .zip(data2.into_iter())
            .flat_map(|(title, mfccs_vec)| {
                mfccs_vec
                    .into_iter()
                    .enumerate()
                    .map(move |(index, mfccs)| {
                        let title = format!("{title}-{index}");
                        (title, mfccs)
                    })
            })
            .collect::<Vec<_>>();
        println!(
            "Flattened {}, elapsed {}ms",
            data4.len(),
            op_timer.elapsed().as_millis()
        );

        // 6. Plot
        // println!("Plotting...");
        // let op_timer = Instant::now();
        // data4
        //     .par_iter()
        //     .for_each(|(title, mfccs)| mfcc::plot(mfccs, title));
        // println!("Plotted, elapsed {}ms", op_timer.elapsed().as_millis());

        // 6. Save binaries.
        println!("Save binaries...");
        let op_timer = Instant::now();
        data4.par_iter().for_each(|(title, mfccs)| {
            let encoded = bincode::serialize(mfccs).unwrap();
            File::create(format!("bins/{title}.bin"))
                .unwrap()
                .write_all(&encoded)
                .unwrap();
        });
        println!("Saved, elapsed {}ms", op_timer.elapsed().as_millis());

        println!("Step complete {}ms", step_timer.elapsed().as_millis());
    }
    println!("Complete {}ms", total_timer.elapsed().as_millis());
    Ok(())
}

struct AudioData {
    content: Vec<u8>,
    kind: ContentKind,
    title: String,
}

impl From<MetadataWithAudio> for AudioData {
    fn from(value: MetadataWithAudio) -> Self {
        AudioData {
            content: value.content,
            kind: value.kind,
            title: format!(
                "{}|{}|{}|{}",
                value.kind.to_string(),
                value.artist,
                value.title,
                value.id
            )
            .replace(['/', '\\', '"', '\'', '.', '&'], ""),
        }
    }
}

fn fetch_data(skip: usize, limit: usize) -> anyhow::Result<Vec<AudioData>> {
    let mut url = Url::parse(ENDPOINT)?.join("/api/v1/segments/msgpack")?;

    url.set_query(Some(&format!("skip={skip}&limit={limit}")));

    let response = Client::new()
        .get(url)
        .header(reqwest::header::ACCEPT, MSGPACK_MIME)
        .send()
        .context("Sending reqwest")?;

    match response.status() {
        StatusCode::OK | StatusCode::CREATED => rmp_serde::from_read(response.bytes()?.reader())
            .map_err(|e| e.into())
            .map(|docs: Vec<MetadataWithAudio>| {
                docs.into_iter().map(|doc| doc.into()).collect::<Vec<_>>()
            }),
        _ => bail!("{} {}", response.status(), response.text()?),
    }
}

fn calculate_mfccs(data: &RawAudioData) -> anyhow::Result<Vec<mfcc::MFCCs>> {
    let window_size = mfcc::SAMPLE_RATE as usize * 4; // 4 secs
    let window_step = mfcc::SAMPLE_RATE as usize; // 1 sec
    let steps = ((data.len() - window_size) as f32 / window_step as f32).round() as usize + 1;
    let mut mels = Vec::new();
    for step in 0..steps {
        let window_start = step * window_step;
        let window_end = (window_start + window_size).min(data.len());
        let data = data.slice(s![window_start..window_end]).to_owned();
        let mfccs = calculate_mel_coefficients_with_deltas(&data)?;
        // The row number varies around 171/172, so lets take the least.`
        let mfccs = mfccs.slice(s![..171, ..]).to_owned();
        mels.push(mfccs);
    }
    Ok(mels)
}
