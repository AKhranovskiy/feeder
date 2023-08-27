use std::time::Duration;

use classifier::PredictedLabels;
use log::info;
use ndarray::{Axis, Slice};
use ndarray_stats::QuantileExt;

use crate::analyzer::DRAIN_DURATION;

#[allow(dead_code)]
pub struct LabelSmoother {
    behind: usize,
    ahead: usize,
    buffer: ndarray::Array3<f32>,
    max_size: usize,
    ads_label: PredictedLabels,
    music_label: PredictedLabels,
    talk_label: PredictedLabels,
    last: PredictedLabels,
}

const RATIO_THRESHOLD: f32 = 0.66;
const ACCURACY_THRESHOLD: f32 = 0.60;

impl LabelSmoother {
    #[must_use]
    pub fn new(behind: Duration, ahead: Duration) -> Self {
        let behind_size = (behind.as_millis() / DRAIN_DURATION.as_millis()) as usize;
        let ahead_size = (ahead.as_millis() / DRAIN_DURATION.as_millis()) as usize;

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
            buffer: ndarray::Array3::zeros((0, 1, 3)), //VecDeque::with_capacity(behind_size + ahead_size + 1),
            max_size: behind_size + ahead_size + 1,
            ads_label: PredictedLabels::from_shape_vec((1, 3), vec![1.0, 0.0, 0.0]).unwrap(),
            music_label: PredictedLabels::from_shape_vec((1, 3), vec![0.0, 1.0, 0.0]).unwrap(),
            talk_label: PredictedLabels::from_shape_vec((1, 3), vec![0.0, 0.0, 1.0]).unwrap(),
            last: PredictedLabels::from_shape_vec((1, 3), vec![0.0, 1.0, 0.0]).unwrap(),
        }
    }

    #[must_use]
    pub fn get_buffer_content(&self) -> String {
        self.buffer
            .axis_iter(Axis(0))
            .map(|item| "#-.".chars().nth(item.argmax().unwrap().1).unwrap_or('_'))
            .collect::<String>()
    }

    pub fn push(&mut self, labels: &PredictedLabels) -> Option<PredictedLabels> {
        self.buffer
            .append(Axis(0), labels.view().into_shape((1, 1, 3)).ok()?)
            .unwrap();

        if self.buffer.shape()[0] < self.ahead {
            return None;
        }

        if let Some(start) = self.buffer.shape()[0].checked_sub(self.ahead) {
            self.buffer
                .slice_axis_inplace(Axis(0), Slice::from(start..));
        }

        self.buffer.mean_axis(Axis(0))
    }
}
