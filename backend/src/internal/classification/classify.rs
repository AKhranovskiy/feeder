use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context};
use mfcc::{ffmpeg_decode, SAMPLE_RATE};
use ndarray::s;
use tch::nn::{ModuleT, SequentialT};
use tch::{IndexOp, Tensor};

use trainer::networks::Network;
use trainer::utils::data::IMAGE_TENSOR_SHAPE;

use crate::internal::prediction::Prediction;

use super::score::Score;
use super::INPUT_CHUNK_DURATION_SEC;
pub fn classify<S: Score>(data: &[u8], score: S) -> anyhow::Result<Vec<Prediction>> {
    let raw_data: mfcc::RawAudioData = ffmpeg_decode(data)?;

    if raw_data.len() < SAMPLE_RATE as usize * INPUT_CHUNK_DURATION_SEC {
        bail!("Audio data is too short, expected at least {INPUT_CHUNK_DURATION_SEC} seconds, given={:01}", raw_data.len() as f32 / SAMPLE_RATE as f32);
    }
    let mfccs = calculate_mfccs(&raw_data).context("Calculating MFCCs")?;

    let images = Tensor::zeros(&[mfccs.len() as i64, 1, 39, 171], tch::kind::FLOAT_CPU);
    for (idx, m) in mfccs.iter().enumerate() {
        // Require standard memory layout to guarantee the correct slice.
        let image =
            Tensor::try_from(m.as_standard_layout()).context("Converting ndarray to Tensor")?;
        let image = image.transpose(1, 1).reshape(&IMAGE_TENSOR_SHAPE);
        images.i(idx as i64).copy_(&image);
    }

    let result = Classifier::new()
        .context("Configuring classificator")?
        .classify(&images)
        .context("Classification")?;
    Ok(score.calculate(&result))
}

struct Classifier {
    net: SequentialT,
}

impl Classifier {
    fn new() -> anyhow::Result<Self> {
        let model_path = env::var("CLASSIFIER_MODEL_PATH").context("Getting model path")?;
        let model_path = PathBuf::from_str(&model_path).context("Validating model path")?;

        if !model_path.exists() {
            bail!("Model path does not exists, path={}", model_path.display())
        }

        let network_name = env::var("CLASSIFIER_NETWORK_NAME")
            .or_else(|_| {
                model_path
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .and_then(|s| s.split('.').nth(1))
                    .map(|s| s.to_owned())
                    .ok_or_else(|| {
                        anyhow!(
                            "Failed to get the network name from the model name, model={}",
                            model_path.display()
                        )
                    })
            })
            .context("Getting network name")?;

        let network = match network_name.to_lowercase().as_str() {
            "cnnpp" => Network::CnnPp,
            _ => bail!("Unknown network name, name={}", network_name),
        };

        let (vs, loaded) = network.create_varstore(&model_path);
        loaded.context("Loading weights")?;

        Ok(Self {
            net: network.create_network(&vs.root()),
        })
    }

    fn classify(&self, images: &Tensor) -> anyhow::Result<Tensor> {
        let labels = tch::Tensor::zeros(&[images.size()[0], 3], tch::kind::FLOAT_CPU);

        for (idx, image) in images.split(1, 0).iter().enumerate() {
            labels.i(idx as i64).copy_(
                &self
                    .net
                    .forward_t(image, /*train=*/ false)
                    .softmax(-1, tch::Kind::Float)
                    .squeeze(),
            );
        }

        Ok(labels)
    }
}

fn calculate_mfccs(data: &mfcc::RawAudioData) -> anyhow::Result<Vec<mfcc::MFCCs>> {
    let window_size = mfcc::SAMPLE_RATE as usize * INPUT_CHUNK_DURATION_SEC;
    let window_step = mfcc::SAMPLE_RATE as usize; // 1 sec
    let steps = ((data.len() - window_size) as f32 / window_step as f32).round() as usize + 1;
    let mut mels = Vec::new();
    for step in 0..steps {
        let window_start = step * window_step;
        let window_end = (window_start + window_size).min(data.len());
        let data = data.slice(s![window_start..window_end]).to_owned();
        let mfccs = mfcc::calculate_mel_coefficients_with_deltas(&data)?;
        if mfccs.shape()[0] < 171 {
            log::error!("Skip block {step}, too short {:?}", mfccs.shape());
            continue;
        }

        // The row number varies around 171/172, so lets take the least.
        let mfccs = mfccs.slice(s![..171, ..]).to_owned();
        mels.push(mfccs);
    }
    Ok(mels)
}
