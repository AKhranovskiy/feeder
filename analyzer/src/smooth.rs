use std::time::Duration;

use classifier::PredictedLabels;
use lazy_static::lazy_static;
use log::info;
use ndarray::{array, Array1, Array2, Axis, Slice};
use ndarray_stats::{DeviationExt, QuantileExt};

use crate::analyzer::DRAIN_DURATION;

lazy_static! {
    static ref ADVERT: PredictedLabels =
        PredictedLabels::from_shape_vec((1, 3), vec![1.0, 0.0, 0.0]).unwrap();
    static ref MUSIC: PredictedLabels =
        PredictedLabels::from_shape_vec((1, 3), vec![0.0, 1.0, 0.0]).unwrap();
    static ref OTHER: PredictedLabels =
        PredictedLabels::from_shape_vec((1, 3), vec![0.0, 0.0, 1.0]).unwrap();
}

#[allow(dead_code)]
pub struct LabelSmoother {
    ahead: usize,
    buffer: PredictedLabels,
    max_size: usize,
    last: PredictedLabels,
}

impl LabelSmoother {
    #[must_use]
    pub fn new(behind: Duration, ahead: Duration) -> Self {
        let behind_size = (behind.as_millis() / DRAIN_DURATION.as_millis()) as usize;
        let ahead_size = (ahead.as_millis() / DRAIN_DURATION.as_millis()) as usize;

        info!(
            "SMOOTHER behind={}ms/{} ahead={}ms/{}",
            behind.as_millis(),
            behind_size,
            ahead.as_millis(),
            ahead_size
        );

        Self {
            ahead: ahead_size,
            buffer: PredictedLabels::zeros((0, 3)),
            max_size: behind_size + ahead_size + 1,
            last: PredictedLabels::from_shape_vec((1, 3), vec![0.0, 1.0, 0.0]).unwrap(),
        }
    }

    #[must_use]
    pub fn get_buffer_content(&self) -> String {
        self.buffer
            .axis_iter(Axis(0))
            .map(|item| "#-.".chars().nth(item.argmax().unwrap()).unwrap_or('_'))
            .collect::<String>()
    }

    pub fn push(&mut self, labels: &PredictedLabels) -> anyhow::Result<Option<Array1<f32>>> {
        self.buffer.append(Axis(0), dist(labels)?.view())?;

        let len = self.buffer.shape()[0];

        if len < self.ahead {
            return Ok(None);
        }

        if let Some(start) = len.checked_sub(self.max_size) {
            self.buffer
                .slice_axis_inplace(Axis(0), Slice::from(start..));
        }

        let confidence = confidence(&self.buffer);
        if confidence == array![0.0, 0.0, 0.0] {
            return Ok(None);
        }

        // TODO How to eliminate short segments?
        // Second buffer on confidence? ATA -> A, MMMMAAMM -> MMMMMMMMMMM
        Ok(Some(confidence))
    }
}

pub fn dist(labels: &PredictedLabels) -> anyhow::Result<Array2<f32>> {
    let repeat =
        |a: &PredictedLabels| ndarray::concatenate(Axis(0), &[a.view()].repeat(labels.shape()[0]));

    let ads = labels.sq_l2_dist(&repeat(&ADVERT)?)?;
    let music = labels.sq_l2_dist(&repeat(&MUSIC)?)?;
    let other = labels.sq_l2_dist(&repeat(&OTHER)?)?;

    Ok(array![[ads, music, other]])
}

#[must_use]
pub fn confidence(input: &PredictedLabels) -> Array1<f32> {
    let mean = input.mean_axis(Axis(0)).map(|v| 1.0 / v).unwrap();
    let total = mean.sum();
    mean.mapv(|v| (v / (total - v) - 2.0).max(0.0))
}
