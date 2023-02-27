use std::collections::VecDeque;

use classifier::PredictedLabels;
use ndarray_stats::QuantileExt;

pub struct LabelSmoother {
    behind: usize,
    ahead: usize,
    buffer: VecDeque<PredictedLabels>,
}

impl LabelSmoother {
    // Each prediction consumes 1.6-2.4s.
    #[must_use]
    pub fn new(behind: usize, ahead: usize) -> Self {
        Self {
            behind,
            ahead,
            buffer: VecDeque::with_capacity(behind + ahead + 1),
        }
    }

    pub fn push(&mut self, labels: &PredictedLabels) -> Option<PredictedLabels> {
        let dim = labels.dim();

        self.buffer.push_back(labels.max_out());

        let full_size = self.ahead + self.behind + 1;
        if self.buffer.len() == full_size {
            let result = Some(
                self.buffer
                    .iter()
                    .fold(PredictedLabels::zeros(dim), |acc, v| acc + v)
                    .max_out(),
            );
            self.buffer.pop_front();
            result
        } else {
            None
        }
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
    use classifier::PredictedLabels;

    use super::LabelSmoother;

    #[test]
    fn test_empty_history() {
        let mut sut = LabelSmoother::new(5, 0);
        let sample = labels([0.2, 0.3, 0.5]);

        assert!(sut.push(&sample).is_none());
    }

    #[test]
    fn test_short_history() {
        let mut sut = LabelSmoother::new(5, 0);

        sut.push(&labels([0.1, 0.2, 0.3]));
        sut.push(&labels([0.2, 0.3, 0.4]));
        sut.push(&labels([0.3, 0.4, 0.5]));
        sut.push(&labels([0.4, 0.5, 0.6]));
        sut.push(&labels([0.5, 0.6, 0.7]));
        let smoothed = sut.push(&labels([0.6, 0.7, 0.8])).unwrap();

        assert_eq!(labels([0.0, 0.0, 1.0]), smoothed);
    }

    #[test]
    fn test_long_history() {
        let mut sut = LabelSmoother::new(5, 0);

        // Fill the history to threshold
        sut.push(&labels([0.1, 0.2, 0.3]));
        sut.push(&labels([0.2, 0.3, 0.4]));
        sut.push(&labels([0.3, 0.4, 0.5]));
        sut.push(&labels([0.4, 0.5, 0.6]));
        sut.push(&labels([0.5, 0.6, 0.7]));

        // Fill the history beyond threshold
        sut.push(&labels([0.5, 0.6, 0.7]));

        let smoothed = sut.push(&labels([0.6, 0.7, 0.8])).unwrap();

        assert_eq!(labels([0.0, 0.0, 1.0]), smoothed);
    }

    fn labels(values: [f32; 3]) -> PredictedLabels {
        PredictedLabels::from_shape_vec((1, 3), values.to_vec()).unwrap()
    }
}
