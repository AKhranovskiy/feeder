use std::collections::VecDeque;

use classifier::PredictedLabels;

pub struct LabelSmoother {
    size: usize,
    history: VecDeque<PredictedLabels>,
}

impl LabelSmoother {
    #[must_use]
    pub fn new(history_size: usize) -> Self {
        Self {
            size: history_size,
            history: VecDeque::with_capacity(history_size),
        }
    }

    pub fn push(&mut self, labels: PredictedLabels) -> PredictedLabels {
        self.history.push_back(labels);

        if self.history.len() > self.size {
            self.history.drain(0..self.history.len() - self.size);
        }

        let length = self.history.len() as f32;
        assert!(length > 0.0);

        self.history
            .iter()
            .fold(PredictedLabels::zeros((1, 3)), |acc, v| acc + v)
            / length
    }
}

#[cfg(test)]
mod tests {
    use classifier::PredictedLabels;

    use super::LabelSmoother;

    #[test]
    fn test_empty_history() {
        let mut sut = LabelSmoother::new(5);
        let sample = labels([0.2, 0.3, 0.5]);

        let smoothed = sut.push(sample.clone());

        assert_eq!(sample, smoothed);
    }

    #[test]
    fn test_short_history() {
        let mut sut = LabelSmoother::new(5);

        sut.push(labels([0.1, 0.2, 0.3]));
        sut.push(labels([0.2, 0.3, 0.4]));
        sut.push(labels([0.3, 0.4, 0.5]));
        sut.push(labels([0.4, 0.5, 0.6]));
        let smoothed = sut.push(labels([0.5, 0.6, 0.7]));

        assert_eq!(labels([0.3, 0.4, 0.5]), smoothed);
    }

    #[test]
    fn test_long_history() {
        let mut sut = LabelSmoother::new(5);

        // Fill the history to threshold
        sut.push(labels([0.1, 0.2, 0.3]));
        sut.push(labels([0.2, 0.3, 0.4]));
        sut.push(labels([0.3, 0.4, 0.5]));
        sut.push(labels([0.4, 0.5, 0.6]));
        sut.push(labels([0.5, 0.6, 0.7]));

        // Fill the history beyond threshold
        sut.push(labels([0.5, 0.6, 0.7]));

        let smoothed = sut.push(labels([0.6, 0.7, 0.8]));

        assert_eq!(labels([0.460_000_04, 2.8 / 5.0, 3.3 / 5.0]), smoothed);
    }

    fn labels(values: [f32; 3]) -> PredictedLabels {
        PredictedLabels::from_shape_vec((1, 3), values.to_vec()).unwrap()
    }
}
