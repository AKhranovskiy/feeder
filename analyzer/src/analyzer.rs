use std::{collections::VecDeque, time::Duration, time::Instant};

use ndarray_stats::QuantileExt;

use classifier::Classifier;
use codec::{resample_16k_mono_s16_frame, AudioFrame, FrameDuration, Timestamp};

use crate::{ContentKind, LabelSmoother};

pub struct BufferedAnalyzer {
    samples_queue: VecDeque<i16>,
    classifer: Classifier,
    smoother: LabelSmoother,
    last_kind: ContentKind,
    print_buffer_stat: bool,
    input_queue: VecDeque<AudioFrame>,
    output_queue: VecDeque<(ContentKind, AudioFrame)>,
    ads_duration: Duration,
    ads_counter: usize,
}

const FRAME_WIDTH: usize = 15_600; // 16_000 * 0.975s
const DRAIN_WIDTH: usize = 1_600; // 16_000 * 0.1s

impl BufferedAnalyzer {
    pub const DRAIN_DURATION: Duration = Duration::from_millis(100);

    pub fn warmup() {
        Classifier::new().expect("Empty model");
    }

    #[must_use]
    pub fn new(smoother: LabelSmoother, print_buffer_stat: bool) -> Self {
        Self {
            samples_queue: VecDeque::new(),
            classifer: Classifier::from_file("./models/adbanda_at_m")
                .expect("Initialized classifier"),
            input_queue: VecDeque::new(),
            output_queue: VecDeque::new(),
            smoother,
            last_kind: ContentKind::Unknown,
            print_buffer_stat,
            ads_duration: Duration::default(),
            ads_counter: 0,
        }
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<Option<(ContentKind, AudioFrame)>> {
        let zero_pts = Timestamp::new(0, frame.time_base());
        let frame_duration = frame.duration();

        let timer = Instant::now();

        self.samples_queue
            .extend(resample_16k_mono_s16_frame(frame.clone())?.into_iter());
        self.input_queue.push_back(frame);

        if self.samples_queue.len() >= FRAME_WIDTH {
            let samples = self
                .samples_queue
                .iter()
                .take(FRAME_WIDTH)
                .copied()
                .collect::<Vec<_>>();

            self.samples_queue.drain(0..DRAIN_WIDTH);

            let data = classifier::Data::from_shape_vec((1, FRAME_WIDTH), samples)?;
            let prediction = self.classifer.predict(&data)?;

            if let Some(smoothed) = self.smoother.push(prediction) {
                let kind = match smoothed.argmax()?.1 {
                    0 => ContentKind::Advertisement,
                    1 => ContentKind::Music,
                    2 => ContentKind::Talk,
                    x => unreachable!("Unexpected label {x}"),
                };

                let frames_to_drain = self
                    .input_queue
                    .iter()
                    .scan(Duration::ZERO, |acc, frame| {
                        *acc += frame.duration();
                        Some(*acc)
                    })
                    .take_while(|dur| dur < &Self::DRAIN_DURATION)
                    .count();

                self.output_queue.extend(
                    self.input_queue
                        .drain(0..frames_to_drain)
                        .map(|frame| (kind, frame)),
                );
            }
        }

        if let Some((kind, frame)) = self.output_queue.front() {
            if *kind == ContentKind::Advertisement {
                self.ads_duration += frame.duration();
                if *kind != self.last_kind {
                    self.ads_counter += 1;
                }
            }
            self.last_kind = *kind;
        }

        if self.print_buffer_stat {
            print!(
                "\r{:<3}ms, {:?}: {} {:#} {}s/{}          ",
                timer.elapsed().as_millis(),
                self.output_queue
                    .front()
                    .as_ref()
                    .map_or(zero_pts, |(_, frame)| frame.pts()),
                self.smoother.get_buffer_content(),
                self.output_queue
                    .front()
                    .map_or(ContentKind::Unknown, |(kind, _)| *kind),
                self.ads_duration.as_secs(),
                self.ads_counter
            );
        }

        if timer.elapsed() >= frame_duration {
            log::error!(
                "Frame processing time exceeded frame duration: {}ms vs {}ms",
                timer.elapsed().as_millis(),
                frame_duration.as_millis()
            );
        }
        Ok(self.output_queue.pop_front())
    }
}
