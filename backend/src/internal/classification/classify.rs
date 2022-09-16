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

