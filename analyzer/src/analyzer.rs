use std::collections::VecDeque;

use anyhow::anyhow;

use bytemuck::cast_slice;
use classifier::Classifier;
use codec::{AudioFrame, CodecParams, Resampler, SampleFormat};
use ndarray_stats::QuantileExt;

use crate::LabelSmoother;

pub struct BufferedAnalyzer {
    queue: VecDeque<f64>,
    classifer: Classifier,
    smoother: LabelSmoother,
    last_class: Option<&'static str>,
}

impl BufferedAnalyzer {
    pub fn new(smoother: LabelSmoother) -> Self {
        Self {
            queue: VecDeque::with_capacity(150 * 39 * 2),
            classifer: Classifier::from_file("./model").expect("Initialized classifier"),
            smoother,
            last_class: None,
        }
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<Option<&'static str>> {
        if frame.samples() < 128 {
            return Ok(self.last_class);
        }

        let mut samples: Vec<f32> = vec![];

        resample(frame)?.planes().iter().for_each(|plane| {
            samples.extend_from_slice(cast_slice(plane.data()));
        });

        let coeffs = mfcc::Config::default().num_coefficients;

        let mut mfccs = mfcc::calculate_mfccs(samples.as_slice(), Default::default())?
            .into_iter()
            .map(f64::from)
            .collect::<VecDeque<_>>();

        self.queue.append(&mut mfccs);

        if self.queue.len() >= (150 * coeffs) {
            let data = self
                .queue
                .iter()
                .take(150 * coeffs)
                .cloned()
                .collect::<Vec<_>>();

            let data = ndarray::Array4::from_shape_vec((1, 150, coeffs, 1), data)?;

            self.queue.drain(..(100 * coeffs));

            let prediction = self.classifer.predict(&data)?;
            let prediction = self.smoother.push(prediction);

            self.last_class = match prediction.argmax()?.1 {
                0 => Some("A"),
                1 => Some("M"),
                2 => Some("T"),
                _ => unreachable!("Unexpected prediction shape"),
            };
        }

        Ok(self.last_class)
    }
}

const MFCCS_CODEC_PARAMS: CodecParams = CodecParams::new(22050, SampleFormat::Flt, 1);

fn resample(frame: AudioFrame) -> anyhow::Result<AudioFrame> {
    let mut resampler = Resampler::new(CodecParams::from(&frame), MFCCS_CODEC_PARAMS);

    resampler
        .push(frame)?
        .next()
        .transpose()?
        .ok_or_else(|| anyhow!("Resampler returns no data"))
}
