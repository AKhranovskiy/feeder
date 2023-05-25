use std::{collections::VecDeque, time::Duration, time::Instant};

use ac_ffmpeg::time::TimeBase;
use bytemuck::cast_slice;
use ndarray_stats::QuantileExt;

use classifier::Classifier;
use codec::{AudioFrame, CodecParams, FrameDuration, Resampler, SampleFormat, Timestamp};

use crate::{ContentKind, LabelSmoother};

pub struct BufferedAnalyzer {
    queue: VecDeque<i16>,
    classifer: Classifier,
    smoother: LabelSmoother,
    last_kind: ContentKind,
    print_buffer_stat: bool,
    frame_buffer: VecDeque<AudioFrame>,
    ads_duration: Duration,
    ads_counter: usize,
}

const FRAME_WIDTH: usize = 15_600; // 16_000 * 0.975ms
const DRAIN_WIDTH: usize = 7_680; // 16_000 * 0.100ms

impl BufferedAnalyzer {
    pub const DRAIN_DURATION: Duration = Duration::from_millis(100);

    pub fn warmup() {
        Classifier::new().expect("Empty model");
    }

    #[must_use]
    pub fn new(smoother: LabelSmoother, print_buffer_stat: bool) -> Self {
        Self {
            queue: VecDeque::with_capacity(2 * 16_000),
            classifer: Classifier::from_file("./models/adbanda").expect("Initialized classifier"),
            frame_buffer: VecDeque::with_capacity(smoother.ahead()),
            smoother,
            last_kind: ContentKind::Unknown,
            print_buffer_stat,
            ads_duration: Duration::default(),
            ads_counter: 0,
        }
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<Option<(ContentKind, AudioFrame)>> {
        let samples = samples(frame.clone())?;
        self.queue.extend(samples.into_iter());
        self.frame_buffer.push_back(frame);

        let elapsed = if self.queue.len() >= FRAME_WIDTH {
            let timer = Instant::now();
            let samples = self
                .queue
                .iter()
                .take(FRAME_WIDTH)
                .copied()
                .collect::<Vec<_>>();

            self.queue.drain(0..DRAIN_WIDTH);

            let data = classifier::Data::from_shape_vec((1, FRAME_WIDTH), samples)?;
            let prediction = self.classifer.predict(&data)?;

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

fn samples(frame: AudioFrame) -> anyhow::Result<Vec<i16>> {
    const CODEC_PARAMS: CodecParams = CodecParams::new(16_000, SampleFormat::S16, 1);

    dbg!(CodecParams::from(&frame));
    let mut resampler = Resampler::new(CodecParams::from(&frame), CODEC_PARAMS);

    let mut output = Vec::with_capacity(frame.samples());

    for frame in resampler.push(frame)? {
        for plane in frame?.planes().iter() {
            output.extend_from_slice(cast_slice(plane.data()));
        }
    }

    Ok(output)
}
