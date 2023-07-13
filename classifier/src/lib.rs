use std::sync::Mutex;

mod pyvtable;
mod types;

use self::pyvtable::PyVTable;
use self::types::PyModel;
pub use self::types::{Data, Labels, PredictedLabels};

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

    pub fn check() -> anyhow::Result<()> {
        PyVTable::check()
    }

    pub fn predict(&self, data: &Data) -> anyhow::Result<PredictedLabels> {
        let model = self.model.lock().unwrap();
        PyVTable::predict(&model, data)
    }
}
