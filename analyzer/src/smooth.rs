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
}

const ADS_RATIO_THRESHOLD: f32 = 0.75;
const ADS_ACCURACY_THRESHOLD: f32 = 0.70;

impl LabelSmoother {
    #[must_use]
    pub fn new(behind: Duration, ahead: Duration) -> Self {
        let behind_size =
            (behind.as_millis() / BufferedAnalyzer::DRAIN_DURATION.as_millis()) as usize;
        let ahead_size =
            (ahead.as_millis() / BufferedAnalyzer::DRAIN_DURATION.as_millis()) as usize;

        info!(
            "SMOOTHER behind={}ms/{} ahead={}ms/{} accuracy={ADS_ACCURACY_THRESHOLD} ratio={ADS_RATIO_THRESHOLD}",
            behind.as_millis(),
            behind_size,
            ahead.as_millis(),
            ahead_size
        );

        Self {
            behind: behind_size,
            ahead: ahead_size,
            buffer: VecDeque::with_capacity(behind_size + ahead_size + 1),
            ads_label: PredictedLabels::from_shape_vec((1, 2), vec![1.0, 0.0]).unwrap(),
            music_label: PredictedLabels::from_shape_vec((1, 2), vec![0.0, 1.0]).unwrap(),
        }
    }

    #[must_use]
    pub fn get_buffer_content(&self) -> String {
        format!(
            "{} {:.2}",
            self.buffer
                .iter()
                .map(|item| { "#-.".chars().nth(item.argmax().unwrap().1).unwrap_or('_') })
                .collect::<String>(),
            self.get_ads_ratio()
        )
    }

    fn get_ads_ratio(&self) -> f32 {
        let ads = self
            .buffer
            .iter()
            .filter(|x| x[(0, 0)] > ADS_ACCURACY_THRESHOLD)
            .count();

        // TODO count talks
        ads as f32 / self.buffer.len() as f32
    }

    pub fn push(&mut self, labels: PredictedLabels) -> Option<PredictedLabels> {
        // Some(labels)
        self.buffer.push_back(labels);

        if self.buffer.len() < self.ahead {
            return None;
        }

        if self.buffer.len() > (self.ahead + self.behind + 1) {
            self.buffer.pop_front();
        }

        if self.get_ads_ratio() > ADS_RATIO_THRESHOLD {
            Some(self.ads_label.clone())
        } else {
            Some(self.music_label.clone())
        }
    }

    pub(crate) fn ahead(&self) -> usize {
        self.ahead
    }
}
