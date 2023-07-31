use anyhow::anyhow;
use std::{path::Path, sync::Mutex};

mod pyvtable;
mod types;

use self::pyvtable::PyVTable;
use self::types::PyModel;
pub use self::types::{Data, Labels, PredictedLabels};

#[inline(never)]
pub fn check_gpu(required: bool) -> anyhow::Result<()> {
    // Ensure num_gpus is always called first
    (PyVTable::num_gpus()? > 0 || !required)
        .then_some(())
        .ok_or_else(|| anyhow!("GPU is not found"))
}

pub trait Classify: Send + Sync {
    fn classify(&self, data: &Data) -> anyhow::Result<PredictedLabels>;
}

#[derive(Debug, Clone, Copy)]
pub enum ClassifyModel {
    OneLevel,
    TwoLevels,
}

pub fn create<P>(model: ClassifyModel, dir: P) -> anyhow::Result<Box<dyn Classify>>
where
    P: AsRef<Path>,
{
    Ok(match model {
        ClassifyModel::OneLevel => Box::new(AmtClassifier::load(dir)?),
        ClassifyModel::TwoLevels => Box::new(MoatClassifer::load(dir)?),
    })
}

struct AmtClassifier {
    model: Mutex<PyModel>,
}

impl AmtClassifier {
    fn load<P>(dir: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            model: Mutex::new(PyVTable::load(&dir.as_ref().join("adbanda_atm"))?),
        })
    }
}

impl Classify for AmtClassifier {
    fn classify(&self, data: &Data) -> anyhow::Result<PredictedLabels> {
        let model = self.model.lock().unwrap();
        PyVTable::predict(&model, data)
    }
}

struct MoatClassifer {
    model_mo: Mutex<PyModel>,
    model_at: Mutex<PyModel>,
}

impl MoatClassifer {
    fn load<P>(dir: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            model_mo: Mutex::new(PyVTable::load(&dir.as_ref().join("adbanda_mo"))?),
            model_at: Mutex::new(PyVTable::load(&dir.as_ref().join("adbanda_at"))?),
        })
    }
}

impl Classify for MoatClassifer {
    fn classify(&self, data: &Data) -> anyhow::Result<PredictedLabels> {
        let (music, other) = {
            let model = self.model_mo.lock().unwrap();
            let p = PyVTable::predict(&model, data)?;
            assert_eq!(p.shape(), &[1, 2]);
            (p[(0, 0)], p[(0, 1)])
        };

        let (ads, talk) = {
            let model = self.model_at.lock().unwrap();
            let p = PyVTable::predict(&model, data)?;
            assert_eq!(p.shape(), &[1, 2]);
            (p[(0, 0)], p[(0, 1)])
        };

        Ok(PredictedLabels::from_shape_vec(
            (1, 3),
            vec![other * ads, music, other * talk],
        )?)
    }
}
