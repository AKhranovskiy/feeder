use std::{collections::VecDeque, time::Duration};

use anyhow::anyhow;
use bytemuck::cast_slice;
use ndarray::Array4;
use ndarray_stats::QuantileExt;
use time::{format_description, macros::offset, Instant};

use classifier::Classifier;
use codec::{AudioFrame, CodecParams, Resampler, SampleFormat};

use crate::{ContentKind, LabelSmoother};

pub struct BufferedAnalyzer {
    queue: VecDeque<i16>,
    classifer: Classifier,
    smoother: LabelSmoother,
    last_kind: ContentKind,
    prediction_timer: Instant,
}

impl BufferedAnalyzer {
    pub const DRAIN_DURATION: Duration = mfcc::Config::default().frame_duration();

    const COEFFS: usize = mfcc::Config::default().num_coefficients;

    pub fn warmup() {
        Classifier::new().expect("Empty model");
    }

    #[must_use]
    pub fn new(smoother: LabelSmoother) -> Self {
        Self {
            queue: VecDeque::with_capacity(150 * 39 * 2),
            classifer: Classifier::from_file("./model").expect("Initialized classifier"),
            smoother,
            last_kind: ContentKind::Unknown,
            prediction_timer: Instant::now(),
        }
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<ContentKind> {
        const CONFIG: mfcc::Config = mfcc::Config::default();

        let pts = frame.pts();

        let samples: Vec<i16> = samples(frame)?;
        self.queue.extend(samples.into_iter());

        if self.queue.len() >= 150 * CONFIG.frame_size {
            let samples = self
                .queue
                .iter()
                .take(150 * CONFIG.frame_size)
                .copied()
                .map(f32::from)
                .collect::<Vec<_>>();

            self.queue.drain(0..CONFIG.frame_size);

            let mfccs = mfcc::calculate_mfccs(&samples, mfcc::Config::default())?;
            assert_eq!(150 * Self::COEFFS, mfccs.len(),);
            let mfccs = Array4::from_shape_vec((1, 150, Self::COEFFS, 1), mfccs)?;

            let prediction = self.classifer.predict(&mfccs)?;

            eprint!(
                "{}, {:03}ms, {:?}:",
                time::OffsetDateTime::now_utc()
                    .to_offset(offset!(+8))
                    .format(
                        &format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                            .unwrap()
                    )
                    .unwrap(),
                self.prediction_timer.elapsed().whole_milliseconds(),
                pts
            );

            if let Some(prediction) = self.smoother.push(prediction) {
                self.last_kind = match prediction.argmax()?.1 {
                    0 => ContentKind::Advertisement,
                    1 => ContentKind::Music,
                    2 => ContentKind::Talk,
                    _ => unreachable!("Unexpected prediction shape"),
                };
            }
            eprintln!(" {:#}", self.last_kind);

            self.prediction_timer = Instant::now();
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
