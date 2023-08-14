use std::{
    collections::VecDeque,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    time::Duration,
};

use classifier::{Classify, ClassifyModel};
use enumflags2::BitFlags;
use flume::TryRecvError;
use ndarray_stats::QuantileExt;

use codec::{resample_16k_mono_s16_frame, AudioFrame, FrameDuration, Timestamp};

use crate::{amplify::Apmlify, rate::Rate, AnalyzerOpts, ContentKind, LabelSmoother};

pub struct BufferedAnalyzer {
    frame_sender: flume::Sender<AudioFrame>,
    processed_receiver: flume::Receiver<Vec<(ContentKind, AudioFrame)>>,
    worker_stats_receiver: flume::Receiver<(Duration, String)>,
    stats_sender: flume::Sender<Stats>,
    last_kind: ContentKind,
    output_queue: VecDeque<(ContentKind, AudioFrame)>,
    ads_duration: Duration,
    ads_counter: usize,
    processing_flag: Arc<AtomicBool>,
}

const FRAME_WIDTH: usize = 15_600; // 16_000 * 0.975s
const DRAIN_WIDTH: usize = 1_600; // 16_000 * 0.1s
pub(crate) const DRAIN_DURATION: Duration = Duration::from_millis(100);

const MODEL: ClassifyModel = ClassifyModel::AMT;
// Amplification 2:5:0 does ~72% -> ~50% confidence reduction for Advertisments for AO model.
// Amplification 2:5:5 does ~75 -> ~55% confidence reduction for Advertisments for AMT model.
const AMPLIFICATION: [f32; 3] = [0.25, 1., 1.];

impl BufferedAnalyzer {
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

        let classifier = classifier::create("./models", MODEL).expect("Initialized classifier");

        let processing_flag = Arc::new(AtomicBool::new(false));
        let flag = processing_flag.clone();

        std::thread::spawn(move || {
            processing_worker(
                classifier.as_ref(),
                smoother,
                &frame_receiver,
                &processed_sender,
                &worker_stats_sender,
                &flag,
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
            processing_flag,
        }
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<()> {
        self.frame_sender.send(frame)?;

        Ok(())
    }

    pub fn pop(&mut self) -> anyhow::Result<Vec<(ContentKind, AudioFrame)>> {
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
                frame_duration: frame.duration(),
                pts: frame.pts(),
                kind,
                ads_duration: self.ads_duration,
                ads_counter: self.ads_counter,
            });
        }

        Ok(self.output_queue.drain(..).collect())
    }

    pub fn flush(&mut self) -> anyhow::Result<()> {
        while !self.frame_sender.is_empty() {
            std::thread::yield_now();
        }

        Ok(())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frame_sender.is_empty() && self.output_queue.is_empty()
    }

    #[must_use]
    pub fn is_processing(&self) -> bool {
        self.processing_flag.load(atomic::Ordering::SeqCst)
    }

    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.is_empty() && !self.is_processing()
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
    classifier: &dyn Classify,
    mut smoother: LabelSmoother,
    frame_receiver: &flume::Receiver<AudioFrame>,
    processed_sender: &flume::Sender<Vec<(ContentKind, AudioFrame)>>,
    worker_stats_sender: &flume::Sender<(Duration, String)>,
    processing_flag: &Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut rate = Rate::new();
    let mut samples_queue = VecDeque::<i16>::new();
    let mut input_queue = VecDeque::<AudioFrame>::new();

    while let Ok(frame) = frame_receiver.recv() {
        processing_flag.store(true, atomic::Ordering::SeqCst);
        rate.start();

        input_queue.push_back(frame.clone());
        samples_queue.extend(resample_16k_mono_s16_frame(frame)?);

        while samples_queue.len() >= FRAME_WIDTH {
            let samples = samples_queue
                .iter()
                .take(FRAME_WIDTH)
                .copied()
                .map(f32::from)
                .collect::<Vec<_>>();

            samples_queue.drain(0..DRAIN_WIDTH);

            let data = classifier::Data::from_shape_vec((FRAME_WIDTH,), samples)? / 32768.0;
            let prediction = classifier.classify(&data)?.amplified(&AMPLIFICATION);

            if let Some(smoothed) = smoother.push(prediction) {
                let kind = match smoothed.argmax()?.1 {
                    0 => ContentKind::Advertisement,
                    1 => ContentKind::Music,
                    2 => ContentKind::Talk,
                    x => unreachable!("Unexpected label {x}"),
                };

                // Since frame has been resampled, it needs to count how many original frames
                // fits into DRAIN_DURATION, it is likely to be constant value for a stream.
                let frames_to_drain = input_queue
                    .iter()
                    .scan(Duration::ZERO, |acc, frame| {
                        *acc += frame.duration();
                        Some(*acc)
                    })
                    .take_while(|dur| dur <= &DRAIN_DURATION)
                    .count();

                // It drains 1 more frame than it should, and `input_frames` empties faster than `samples_queue`.
                // The rest of samples queue goes into next cycle, causing small variation of output frames.
                processed_sender.send(
                    input_queue
                        .drain(0..input_queue.len().min(frames_to_drain + 1))
                        .map(|frame| (kind, frame))
                        .collect(),
                )?;
            }
        }

        rate.stop();
        worker_stats_sender.send((rate.average(), smoother.get_buffer_content()))?;
        processing_flag.store(false, atomic::Ordering::SeqCst);
    }

    Ok(())
}
