use std::sync::Mutex;

use ndarray_stats::QuantileExt;

mod pyvtable;
mod types;

use self::pyvtable::PyVTable;
pub use self::types::PredictedLabels;
use self::types::{Data, Labels, PyModel};

#[non_exhaustive]
pub struct Classifier {
    model: Mutex<PyModel>,
}

impl Classifier {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        Ok(Classifier {
            model: Mutex::new(PyVTable::load(path)?),
        })
    }

    pub fn new() -> anyhow::Result<Self> {
        Ok(Classifier {
            model: Mutex::new(PyVTable::define()?),
        })
    }

    pub fn predict(&self, data: &Data) -> anyhow::Result<PredictedLabels> {
        let model = self.model.lock().unwrap();
        PyVTable::predict(&model, data)
    }

    pub fn train(&mut self, data: &Data, labels: &Labels) -> anyhow::Result<()> {
        let mut model = self.model.lock().unwrap();
        *model = PyVTable::train(&model, data, labels)?;
        Ok(())
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let model = self.model.lock().unwrap();
        PyVTable::save(&model, path)
    }
}

pub fn verify(predicted: &PredictedLabels, valid: &Labels) -> anyhow::Result<f32> {
    let labels = predicted
        .rows()
        .into_iter()
        .map(|a| a.argmax().map(|n| n as u32))
        .collect::<Result<Labels, _>>()?;

    let correct = labels
        .iter()
        .zip(valid.iter())
        .filter(|(p, c)| p == c)
        .count();

    Ok((correct as f32) / (labels.len() as f32) * 100.0)
}
