use std::{collections::VecDeque, time::Duration};

use classifier::PredictedLabels;
use log::info;
use ndarray_stats::QuantileExt;

use crate::BufferedAnalyzer;

#[allow(dead_code)]
pub struct LabelSmoother {
    behind: usize,
    ahead: usize,
    buffer: VecDeque<PredictedLabels>,
    ads_label: PredictedLabels,
    music_label: PredictedLabels,
    talk_label: PredictedLabels,
}

const RATIO_THRESHOLD: f32 = 0.75;
const ACCURACY_THRESHOLD: f32 = 0.70;

impl LabelSmoother {
    #[must_use]
    pub fn new(behind: Duration, ahead: Duration) -> Self {
        let behind_size =
            (behind.as_millis() / BufferedAnalyzer::DRAIN_DURATION.as_millis()) as usize;
        let ahead_size =
            (ahead.as_millis() / BufferedAnalyzer::DRAIN_DURATION.as_millis()) as usize;

        info!(
            "SMOOTHER behind={}ms/{} ahead={}ms/{} accuracy={ACCURACY_THRESHOLD} ratio={RATIO_THRESHOLD}",
            behind.as_millis(),
            behind_size,
            ahead.as_millis(),
            ahead_size
        );

        Self {
            behind: behind_size,
            ahead: ahead_size,
            buffer: VecDeque::with_capacity(behind_size + ahead_size + 1),
            ads_label: PredictedLabels::from_shape_vec((1, 3), vec![1.0, 0.0, 0.0]).unwrap(),
            music_label: PredictedLabels::from_shape_vec((1, 3), vec![0.0, 1.0, 0.0]).unwrap(),
            talk_label: PredictedLabels::from_shape_vec((1, 3), vec![0.0, 0.0, 1.0]).unwrap(),
        }
    }

    #[must_use]
    pub fn get_buffer_content(&self) -> String {
        let (ads, music, talk) = self.get_ratio();

        format!(
            "{} {ads:.2} / {music:.2} / {talk:2.}",
            self.buffer
                .iter()
                .map(|item| { "#-.".chars().nth(item.argmax().unwrap().1).unwrap_or('_') })
                .collect::<String>(),
        )
    }

    fn get_ratio(&self) -> (f32, f32, f32) {
        if self.buffer.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let ads = self
            .buffer
            .iter()
            .filter(|x| x[(0, 0)] >= ACCURACY_THRESHOLD)
            .count();

        let music = self
            .buffer
            .iter()
            .filter(|x| x[(0, 1)] >= ACCURACY_THRESHOLD)
            .count();

        let talk = self
            .buffer
            .iter()
            .filter(|x| x[(0, 0)] >= ACCURACY_THRESHOLD)
            .count();

        let len = self.buffer.len() as f32;

        (ads as f32 / len, music as f32 / len, talk as f32 / len)
    }

    pub fn push(&mut self, labels: PredictedLabels) -> Option<PredictedLabels> {
        self.buffer.push_back(labels);

        if self.buffer.len() < self.ahead {
            return None;
        }

        if self.buffer.len() > (self.ahead + self.behind + 1) {
            self.buffer.pop_front();
        }

        let (ads, _, talk) = self.get_ratio();

        if ads >= RATIO_THRESHOLD {
            Some(self.ads_label.clone())
        } else if talk >= RATIO_THRESHOLD {
            Some(self.talk_label.clone())
        } else {
            // Just music by default
            Some(self.music_label.clone())
        }
    }
}
