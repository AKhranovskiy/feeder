use std::{collections::VecDeque, time::Duration};

use enumflags2::BitFlags;
use flume::TryRecvError;
use ndarray_stats::QuantileExt;

use classifier::Classifier;
use codec::{resample_16k_mono_s16_frame, AudioFrame, FrameDuration, Timestamp};

use crate::{rate::Rate, AnalyzerOpts, ContentKind, LabelSmoother};

pub struct BufferedAnalyzer {
    frame_sender: flume::Sender<AudioFrame>,
    processed_receiver: flume::Receiver<Vec<(ContentKind, AudioFrame)>>,
    worker_stats_receiver: flume::Receiver<(Duration, String)>,
    stats_sender: flume::Sender<Stats>,
    last_kind: ContentKind,
    output_queue: VecDeque<(ContentKind, AudioFrame)>,
    ads_duration: Duration,
    ads_counter: usize,
}

const FRAME_WIDTH: usize = 15_600; // 16_000 * 0.975s
const DRAIN_WIDTH: usize = 1_600; // 16_000 * 0.1s
pub(crate) const DRAIN_DURATION: Duration = Duration::from_millis(100);

impl BufferedAnalyzer {
    pub fn warmup() {
        Classifier::check().expect("Check succeed");
    }

    #[must_use]
    pub fn new(smoother: LabelSmoother, opts: BitFlags<AnalyzerOpts>) -> Self {
        // Send frame processing stats to printer thread.
        let (stats_sender, stats_receiver) = flume::unbounded();
        std::thread::spawn(move || stats_worker(&stats_receiver, opts));

        // Send frame to processing thread.
        let (frame_sender, frame_receiver) = flume::unbounded();
        // Receive processed frames from processing thread.
        let (processed_sender, processed_receiver) = flume::unbounded();
        // Receive processing stats.
        let (worker_stats_sender, worker_stats_receiver) = flume::unbounded();

        let classifier =
            Classifier::from_file("./models/adbanda_amt").expect("Initialized classifier");

        std::thread::spawn(move || {
            processing_worker(
                &classifier,
                smoother,
                &frame_receiver,
                &processed_sender,
                &worker_stats_sender,
            )
        });

        Self {
            frame_sender,
            processed_receiver,
            worker_stats_receiver,
            stats_sender,
            output_queue: VecDeque::new(),
            last_kind: ContentKind::Unknown,
            ads_duration: Duration::default(),
            ads_counter: 0,
        }
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<Option<(ContentKind, AudioFrame)>> {
        let frame_duration = frame.duration();

        self.frame_sender.send(frame)?;

        match self.processed_receiver.try_recv() {
            Ok(processed_frames) => {
                self.output_queue.extend(processed_frames);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                anyhow::bail!("Frame processing died");
            }
        }

        // Processing thread sends stats on each frame.
        let stats = match self.worker_stats_receiver.try_recv() {
            Ok(v) => v,
            Err(TryRecvError::Empty) => (Duration::ZERO, "\r".into()),
            Err(TryRecvError::Disconnected) => {
                anyhow::bail!("Processing stats died");
            }
        };

        if let Some((kind, frame)) = self.output_queue.front() {
            let kind = *kind;

            if kind == ContentKind::Advertisement {
                self.ads_duration += frame.duration();
                if kind != self.last_kind {
                    self.ads_counter += 1;
                }
            }
            self.last_kind = kind;

            _ = self.stats_sender.send(Stats {
                rate: stats.0,
                buffer: stats.1,
                frame_duration,
                pts: frame.pts(),
                kind,
                ads_duration: self.ads_duration,
                ads_counter: self.ads_counter,
            });
        }

        Ok(self.output_queue.pop_front())
    }
}

struct Stats {
    rate: Duration,
    frame_duration: Duration,
    pts: Timestamp,
    buffer: String,
    kind: ContentKind,
    ads_duration: Duration,
    ads_counter: usize,
}

fn stats_worker(receiver: &flume::Receiver<Stats>, opts: BitFlags<AnalyzerOpts>) {
    while let Ok(stats) = receiver.recv() {
        if opts.contains(AnalyzerOpts::ShowBufferStatistic) {
            print!(
                "\r{:<3}ms, {:?}: {:#} {} {}s/{}          ",
                stats.rate.as_millis(),
                stats.pts,
                stats.kind,
                stats.buffer,
                stats.ads_duration.as_secs(),
                stats.ads_counter
            );
        }

        if opts.contains(AnalyzerOpts::ReportSlowProcessing) && stats.rate >= stats.frame_duration {
            log::error!(
                "Frame processing time exceeded frame duration: {}ms vs {}ms",
                stats.rate.as_millis(),
                stats.frame_duration.as_millis()
            );
        }
    }
}

fn processing_worker(
    classifier: &Classifier,
    mut smoother: LabelSmoother,
    frame_receiver: &flume::Receiver<AudioFrame>,
    processed_sender: &flume::Sender<Vec<(ContentKind, AudioFrame)>>,
    worker_stats_sender: &flume::Sender<(Duration, String)>,
) -> anyhow::Result<()> {
    let mut rate = Rate::new();
    let mut samples_queue = VecDeque::<i16>::new();
    let mut input_queue = VecDeque::<AudioFrame>::new();

    while let Ok(frame) = frame_receiver.recv() {
        rate.start();

        input_queue.push_back(frame.clone());
        samples_queue.extend(resample_16k_mono_s16_frame(frame)?);

        if samples_queue.len() >= FRAME_WIDTH {
            let samples = samples_queue
                .iter()
                .take(FRAME_WIDTH)
                .copied()
                .collect::<Vec<_>>();

            samples_queue.drain(0..DRAIN_WIDTH);

            let data = classifier::Data::from_shape_vec((1, FRAME_WIDTH), samples)?;
            let prediction = classifier.predict(&data)?;

            if let Some(smoothed) = smoother.push(prediction) {
                let kind = match smoothed.argmax()?.1 {
                    0 => ContentKind::Advertisement,
                    1 => ContentKind::Music,
                    2 => ContentKind::Talk,
                    x => unreachable!("Unexpected label {x}"),
                };

                let frames_to_drain = input_queue
                    .iter()
                    .scan(Duration::ZERO, |acc, frame| {
                        *acc += frame.duration();
                        Some(*acc)
                    })
                    .take_while(|dur| dur < &DRAIN_DURATION)
                    .count();

                processed_sender.send(
                    input_queue
                        .drain(0..frames_to_drain)
                        .map(|frame| (kind, frame))
                        .collect(),
                )?;
            }
        }

        rate.stop();
        worker_stats_sender.send((rate.average(), smoother.get_buffer_content()))?;
    }

    Ok(())
}
