use std::{collections::VecDeque, time::Duration, time::Instant};

use anyhow::anyhow;
use bytemuck::cast_slice;
use log::debug;
use ndarray::Array4;
use ndarray_stats::QuantileExt;

use classifier::Classifier;
use codec::{AudioFrame, CodecParams, Resampler, SampleFormat};

use crate::{ContentKind, LabelSmoother};

pub struct BufferedAnalyzer {
    queue: VecDeque<i16>,
    classifer: Classifier,
    smoother: LabelSmoother,
    last_kind: ContentKind,
}

impl BufferedAnalyzer {
    const MFCCS: mfcc::Config = mfcc::Config::const_default();

    const DRAIN: usize = 1;

    pub const DRAIN_DURATION: Duration =
        Duration::from_millis(Self::MFCCS.frame_duration().as_millis() as u64 * Self::DRAIN as u64);

    const COEFFS: usize = Self::MFCCS.num_coefficients;

    pub fn warmup() {
        Classifier::new().expect("Empty model");
    }

    #[must_use]
    pub fn new(smoother: LabelSmoother) -> Self {
        Self {
            queue: VecDeque::with_capacity(2 * 150 * Self::MFCCS.frame_size),
            classifer: Classifier::from_file("./model").expect("Initialized classifier"),
            smoother,
            last_kind: ContentKind::Unknown,
        }
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<ContentKind> {
        let config = mfcc::Config::default();

        let pts = frame.pts();

        let samples: Vec<i16> = samples(frame)?;
        self.queue.extend(samples.into_iter());

        if self.queue.len() >= 76 * config.frame_size {
            let timer = Instant::now();

            let samples = self
                .queue
                .iter()
                .take(76 * config.frame_size)
                .copied()
                .map(f32::from)
                .collect::<Vec<_>>();

            self.queue.drain(0..Self::DRAIN * config.frame_size);

            let mfccs = {
                let mut mfccs = mfcc::calculate_mfccs(&samples, mfcc::Config::default())?;
                mfccs.truncate(150 * Self::COEFFS);
                assert_eq!(150 * Self::COEFFS, mfccs.len(),);
                Array4::from_shape_vec((1, 150, Self::COEFFS, 1), mfccs)?
            };

            let prediction = self.classifer.predict(&mfccs)?;

            if let Some(prediction) = self.smoother.push(prediction) {
                self.last_kind = match prediction.argmax()?.1 {
                    0 => ContentKind::Advertisement,
                    1 => ContentKind::Music,
                    2 => ContentKind::Talk,
                    _ => unreachable!("Unexpected prediction shape"),
                };
            }

            debug!(
                "{:3}ms, {:?}: {} {:#}",
                timer.elapsed().as_millis(),
                pts,
                self.smoother.get_buffer_content(),
                self.last_kind
            );
        }

        Ok(self.last_kind)
    }
}

fn samples(frame: AudioFrame) -> anyhow::Result<Vec<i16>> {
    const MFCCS_CODEC_PARAMS: CodecParams = CodecParams::new(22050, SampleFormat::S16, 1);

    let mut resampler = Resampler::new(CodecParams::from(&frame), MFCCS_CODEC_PARAMS);

    let frame = resampler
        .push(frame)?
        .next()
        .transpose()?
        .ok_or_else(|| anyhow!("Resampler returns no data"))?;

    let mut output: Vec<i16> = vec![];

    frame.planes().iter().for_each(|plane| {
        output.extend_from_slice(cast_slice(plane.data()));
    });

    Ok(output)
}
