use tch::Tensor;

use crate::prediction::Prediction;
use crate::stat::Stats;

pub trait Score {
    fn calculate(&self, prediction: &Tensor) -> Vec<Prediction>;
}

/// Calculates score per second, averaging values over `INPUT_CHUNK_DURATION_SEC` results.
pub struct AveragePerSecondScore;

impl Score for AveragePerSecondScore {
    fn calculate(&self, prediction: &Tensor) -> Vec<Prediction> {
        let probabilities = Vec::<Vec<f32>>::from(prediction);

        let n = probabilities.len();
        assert!(n > 0);

        let padding = 4 - 1;

        (0..n + padding)
            .map(|sec| {
                let a = sec.checked_sub(padding).unwrap_or_default();
                let b = sec.min(n - 1);

                let stats = (a..=b).map(|idx| &probabilities[idx]).fold(
                    (Stats::new(), Stats::new(), Stats::new()),
                    |accum, values| {
                        (
                            accum.0.push(values[0] as f64),
                            accum.1.push(values[1] as f64),
                            accum.2.push(values[2] as f64),
                        )
                    },
                );
                Prediction::new(
                    stats.0.avg().unwrap_or_default() as f32,
                    stats.1.avg().unwrap_or_default() as f32,
                    stats.2.avg().unwrap_or_default() as f32,
                )
            })
            .collect()
    }
}
