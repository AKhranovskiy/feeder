use std::{collections::VecDeque, time::Duration, time::Instant};

use ac_ffmpeg::time::TimeBase;
use bytemuck::cast_slice;
use ndarray::Array4;
use ndarray_stats::QuantileExt;

use classifier::Classifier;
use codec::{AudioFrame, CodecParams, FrameDuration, Resampler, SampleFormat, Timestamp};

use crate::{ContentKind, LabelSmoother};

pub struct BufferedAnalyzer {
    queue: VecDeque<f32>,
    classifer: Classifier,
    smoother: LabelSmoother,
    last_kind: ContentKind,
    print_buffer_stat: bool,
    frame_buffer: VecDeque<AudioFrame>,
    ads_duration: Duration,
    ads_counter: usize,
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
    pub fn new(smoother: LabelSmoother, print_buffer_stat: bool) -> Self {
        Self {
            queue: VecDeque::with_capacity(2 * 150 * Self::MFCCS.frame_size),
            classifer: Classifier::from_file("./model").expect("Initialized classifier"),
            frame_buffer: VecDeque::with_capacity(smoother.ahead()),
            smoother,
            last_kind: ContentKind::Unknown,
            print_buffer_stat,
            ads_duration: Duration::default(),
            ads_counter: 0,
        }
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<Option<(ContentKind, AudioFrame)>> {
        let config = mfcc::Config::default();

        let samples = samples(frame.clone())?;
        self.queue.extend(samples.into_iter());

        self.frame_buffer.push_back(frame);

        let elapsed = if self.queue.len() >= 76 * config.frame_size {
            let timer = Instant::now();
            let samples = self
                .queue
                .iter()
                .take(76 * config.frame_size)
                .copied()
                .collect::<Vec<_>>();

            self.queue.drain(0..Self::DRAIN * config.frame_size);

            let mfccs = {
                let mut mfccs = mfcc::calculate_mfccs(&samples, mfcc::Config::default())?;
                mfccs.truncate(150 * Self::COEFFS);
                assert_eq!(150 * Self::COEFFS, mfccs.len(),);
                Array4::from_shape_vec((1, 150, Self::COEFFS, 1), mfccs)?
            };

            let prediction = self.classifer.predict(&mfccs)?;
            let is_ad = self.last_kind == ContentKind::Advertisement;
            if let Some(prediction) = self.smoother.push(prediction) {
                self.last_kind = match prediction.argmax()?.1 {
                    0 => ContentKind::Advertisement,
                    1 => ContentKind::Music,
                    x => unreachable!("Unexpected label {x}"),
                };
            }
            if !is_ad && self.last_kind == ContentKind::Advertisement {
                self.ads_counter += 1;
            }
            timer.elapsed().as_millis()
        } else {
            0
        };

        let frame = if self.last_kind == ContentKind::Unknown {
            None
        } else {
            self.frame_buffer.pop_front()
        };

        if self.last_kind == ContentKind::Advertisement && frame.is_some() {
            self.ads_duration += frame.as_ref().unwrap().duration();
        }

        if self.print_buffer_stat {
            print!(
                "\r{:<3}ms, {:?}: {} {:#} {}s/{}          ",
                elapsed,
                frame.as_ref().map_or(
                    Timestamp::new(0, TimeBase::new(1, 1)),
                    codec::AudioFrame::pts
                ),
                self.smoother.get_buffer_content(),
                self.last_kind,
                self.ads_duration.as_secs(),
                self.ads_counter
            );
        }
        Ok(Some(self.last_kind).zip(frame))
    }
}

fn samples(frame: AudioFrame) -> anyhow::Result<Vec<f32>> {
    const MFCCS_CODEC_PARAMS: CodecParams = CodecParams::new(22050, SampleFormat::S16, 1);

    let mut resampler = Resampler::new(CodecParams::from(&frame), MFCCS_CODEC_PARAMS);

    let mut output: Vec<i16> = vec![];

    for frame in resampler.push(frame)? {
        for plane in frame?.planes().iter() {
            output.extend_from_slice(cast_slice(plane.data()));
        }
    }

    Ok(output.into_iter().map(f32::from).collect())
}
