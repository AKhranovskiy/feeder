use std::{collections::VecDeque, time::Duration};

use classifier::PredictedLabels;
use ndarray_stats::QuantileExt;

use crate::BufferedAnalyzer;

pub struct LabelSmoother {
    behind: usize,
    ahead: usize,
    buffer: VecDeque<PredictedLabels>,
}

impl LabelSmoother {
    #[must_use]
    pub fn new(behind: Duration, ahead: Duration) -> Self {
        let behind_size =
            (behind.as_millis() / BufferedAnalyzer::DRAIN_DURATION.as_millis()) as usize;
        let ahead_size =
            (ahead.as_millis() / BufferedAnalyzer::DRAIN_DURATION.as_millis()) as usize;

        eprintln!(
            "SMOOTHER behind={}ms/{} ahead={}ms/{}",
            behind.as_millis(),
            behind_size,
            ahead.as_millis(),
            ahead_size
        );

        Self {
            behind: behind_size,
            ahead: ahead_size,
            buffer: VecDeque::with_capacity(behind_size + ahead_size + 1),
        }
    }

    pub fn push(&mut self, labels: PredictedLabels) -> Option<PredictedLabels> {
        let dim = labels.dim();

        self.buffer.push_back(labels);

        if self.buffer.len() < self.ahead {
            return None;
        }

        if self.buffer.len() == (self.ahead + self.behind + 1) {
            self.buffer.pop_front();
        }

        eprint!(
            "{}",
            self.buffer
                .iter()
                .map(|item| { "#-.".chars().nth(item.argmax().unwrap().1).unwrap_or('_') })
                .collect::<String>()
        );

        // TODO handle -#-#-#-#
        // TODO add timings - music cannot be shorter than X, ads can not be shorter then Y
        Some(
            self.buffer
                .iter()
                .fold(PredictedLabels::zeros(dim), |acc, v| acc + v)
                .max_out(),
        )
    }
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

    use classifier::PredictedLabels;

    use super::LabelSmoother;

    const BEHIND: Duration = Duration::from_millis(1000);
    const AHEAD: Duration = Duration::from_millis(1000);

    #[test]
    fn test_short() {
        let mut sut = LabelSmoother::new(BEHIND, AHEAD);

        assert!(sut.push(labels([0.1, 0.2, 0.3])).is_none());
    }

    #[test]
    fn test_exact() {
        let mut sut = LabelSmoother::new(BEHIND, AHEAD);

        sut.push(labels([0.1, 0.2, 0.3]));
        sut.push(labels([0.2, 0.3, 0.4]));
        sut.push(labels([0.3, 0.4, 0.5]));
        let smoothed = sut.push(labels([0.6, 0.7, 0.8])).unwrap();

        assert_eq!(labels([0.0, 0.0, 1.0]), smoothed);
    }

    #[test]
    fn test_long_history() {
        let mut sut = LabelSmoother::new(BEHIND, AHEAD);

        // Fill the history to threshold
        sut.push(labels([0.1, 0.2, 0.3]));
        sut.push(labels([0.2, 0.3, 0.4]));
        sut.push(labels([0.3, 0.4, 0.5]));
        sut.push(labels([0.4, 0.5, 0.6]));

        let smoothed = sut.push(labels([0.6, 0.7, 0.8])).unwrap();

        assert_eq!(labels([0.0, 0.0, 1.0]), smoothed);
    }

    fn labels(values: [f32; 3]) -> PredictedLabels {
        PredictedLabels::from_shape_vec((1, 3), values.to_vec()).unwrap()
    }
}
