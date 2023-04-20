use std::{collections::VecDeque, time::Duration};

use classifier::PredictedLabels;
use log::info;
use ndarray_stats::QuantileExt;

use crate::BufferedAnalyzer;

pub struct LabelSmoother {
    behind: usize,
    ahead: usize,
    buffer: VecDeque<PredictedLabels>,
    ads_threshold: f32,
}

impl LabelSmoother {
    #[must_use]
    pub fn new(behind: Duration, ahead: Duration) -> Self {
        // let behind = behind.max(Duration::from_millis(1450)) - Duration::from_millis(1450);
        // let ahead = ahead.max(Duration::from_millis(1450)) - Duration::from_millis(1450);

        let behind_size =
            (behind.as_millis() / BufferedAnalyzer::DRAIN_DURATION.as_millis() / 2) as usize;
        let ahead_size =
            (ahead.as_millis() / BufferedAnalyzer::DRAIN_DURATION.as_millis() / 2) as usize;

        let ads_threshold = if ahead_size > 0 && behind_size > 0 {
            ahead_size as f32 / (behind_size + ahead_size) as f32
        } else {
            0.0
        };

        info!(
            "SMOOTHER behind={}ms/{} ahead={}ms/{} threshold={ads_threshold}",
            behind.as_millis(),
            behind_size,
            ahead.as_millis(),
            ahead_size
        );

        Self {
            behind: behind_size,
            ahead: ahead_size,
            buffer: VecDeque::with_capacity(behind_size + ahead_size + 1),
            ads_threshold,
        }
    }

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
            .map(MaxOutExt::max_out)
            .filter(|x| (x[(0, 0)] - 1.0).abs() < f32::EPSILON)
            .count();

        // TODO count talks
        ads as f32 / self.buffer.len() as f32
    }

    pub fn push(&mut self, labels: PredictedLabels) -> Option<PredictedLabels> {
        self.buffer.push_back(labels);

        if self.buffer.len() < self.ahead {
            return None;
        }

        if self.buffer.len() == (self.ahead + self.behind + 1) {
            self.buffer.pop_front();
        }

        if self.get_ads_ratio() > self.ads_threshold {
            Some(make_labels(1.0, 0.0))
        } else {
            Some(make_labels(0.0, 1.0))
        }
    }

    pub(crate) fn ahead(&self) -> usize {
        self.ahead
    }
}

fn make_labels(ads: f32, music: f32) -> PredictedLabels {
    PredictedLabels::from_shape_vec((1, 3), vec![ads, music, 0.0]).unwrap()
}

trait MaxOutExt {
    fn max_out(&self) -> Self;
}

impl MaxOutExt for PredictedLabels {
    fn max_out(&self) -> Self {
        if let Ok(max_value) = self.max() {
            self.map(|v| if v < max_value { 0.0 } else { 1.0 })
        } else {
            self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::make_labels;
    use super::LabelSmoother;

    const BEHIND: Duration = Duration::from_millis(40);
    const AHEAD: Duration = Duration::from_millis(20);

    #[test]
    fn test_short() {
        let mut sut = LabelSmoother::new(BEHIND, AHEAD);

        assert_eq!(
            sut.push(make_labels(0.6, 0.4)).unwrap(),
            make_labels(1.0, 0.0)
        );
    }

    #[test]
    fn test_exact() {
        let mut sut = LabelSmoother::new(BEHIND, AHEAD);

        sut.push(make_labels(0.6, 0.4));
        sut.push(make_labels(0.2, 0.8));
        let smoothed = sut.push(make_labels(0.4, 0.6)).unwrap();

        assert_eq!(make_labels(0.0, 1.0), smoothed);
    }

    #[test]
    fn test_long_history() {
        let mut sut = LabelSmoother::new(BEHIND, AHEAD);

        sut.push(make_labels(0.4, 0.6));
        sut.push(make_labels(0.4, 0.6));
        sut.push(make_labels(0.4, 0.6));
        sut.push(make_labels(0.6, 0.6));

        let smoothed = sut.push(make_labels(0.6, 0.4)).unwrap();

        assert_eq!(make_labels(1.0, 0.0), smoothed);
    }
}
