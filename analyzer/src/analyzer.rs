use std::{collections::VecDeque, time::Duration};

use anyhow::anyhow;

use bytemuck::cast_slice;
use classifier::Classifier;
use codec::{AudioFrame, CodecParams, Resampler, SampleFormat};
use ndarray_stats::QuantileExt;
use time::{format_description, macros::offset, Instant};

use crate::{ContentKind, LabelSmoother};

pub struct BufferedAnalyzer {
    queue: VecDeque<f64>,
    classifer: Classifier,
    smoother: LabelSmoother,
    last_kind: ContentKind,
    prediction_timer: Instant,
}

impl BufferedAnalyzer {
    pub const DRAIN_DURATION: Duration = Duration::from_millis(300);
    const DRAIN_COEFFS: usize = Self::DRAIN_DURATION.as_millis() as usize
        / mfcc::Config::default().frame_duration().as_millis() as usize;

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
        if frame.samples() < 128 {
            return Ok(self.last_kind);
        }
        let pts = frame.pts();

        let mut samples: Vec<f32> = vec![];

        resample(frame)?.planes().iter().for_each(|plane| {
            samples.extend_from_slice(cast_slice(plane.data()));
        });

        let coeffs = mfcc::Config::default().num_coefficients;

        let mut mfccs = mfcc::calculate_mfccs(samples.as_slice(), mfcc::Config::default())?
            .into_iter()
            .map(f64::from)
            .collect::<VecDeque<_>>();

        self.queue.append(&mut mfccs);

        if self.queue.len() >= (150 * coeffs) {
            let data = self
                .queue
                .iter()
                .take(150 * coeffs)
                .copied()
                .collect::<Vec<_>>();

            let data = ndarray::Array4::from_shape_vec((1, 150, coeffs, 1), data)?;

            // 1 coeff block is 20ms
            // Drain 500ms
            self.queue.drain(..Self::DRAIN_COEFFS * coeffs);

            let prediction = self.classifer.predict(&data)?;

            eprint!(
                "{}, {:03}ms, {:?}: ",
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

const MFCCS_CODEC_PARAMS: CodecParams = CodecParams::new(22050, SampleFormat::Flt, 1);

fn resample(frame: AudioFrame) -> anyhow::Result<AudioFrame> {
    let mut resampler = Resampler::new(CodecParams::from(&frame), MFCCS_CODEC_PARAMS);

    resampler
        .push(frame)?
        .next()
        .transpose()?
        .ok_or_else(|| anyhow!("Resampler returns no data"))
}
