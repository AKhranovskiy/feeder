use std::path::Path;

use tensorflow::Tensor;

mod tfmodel;
mod types;

pub use self::types::{Data, Labels, PredictedLabels};
use tfmodel::TfModel;

pub trait Classify: Send + Sync {
    fn classify(&self, data: &Data) -> anyhow::Result<PredictedLabels>;
}

#[derive(Debug, Clone, Copy)]
pub enum ClassifyModel {
    AMT,
    MOAT,
    AO,
}

pub fn create<P: AsRef<Path>>(dir: P, model: ClassifyModel) -> anyhow::Result<Box<dyn Classify>> {
    match model {
        ClassifyModel::AMT => Ok(Box::new(AmtClassifier::load(dir)?)),
        ClassifyModel::MOAT => Ok(Box::new(MoatClassifier::load(dir)?)),
        ClassifyModel::AO => Ok(Box::new(AoClassifier::load(dir)?)),
    }
}

struct AmtClassifier {
    yamnet: TfModel,
    adbanda: TfModel,
}

impl AmtClassifier {
    fn load<P>(dir: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            yamnet: TfModel::yamnet(&dir)?,
            adbanda: TfModel::adbanda(&dir, "adbanda_amt")?,
        })
    }
}

impl Classify for AmtClassifier {
    fn classify(&self, data: &Data) -> anyhow::Result<PredictedLabels> {
        let embedding = self.yamnet.run(&Tensor::from(data))?;
        let prediction = self.adbanda.run(&embedding)?;

        assert_eq!(prediction.shape(), [1, 3].into());
        let (ads, music, talk) = (prediction[0], prediction[1], prediction[2]);
        Ok(PredictedLabels::from_shape_vec(
            (1, 3),
            vec![ads, music, talk],
        )?)
    }
}

struct AoClassifier {
    yamnet: TfModel,
    adbanda: TfModel,
}

impl AoClassifier {
    fn load<P>(dir: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let yamnet = TfModel::yamnet(&dir)?;
        let adbanda = TfModel::adbanda(&dir, "adbanda_ao/")?;

        Ok(Self { yamnet, adbanda })
    }
}

impl Classify for AoClassifier {
    fn classify(&self, data: &Data) -> anyhow::Result<PredictedLabels> {
        let (ads, other) = {
            let embedding: Tensor<f32> = self.yamnet.run(&Tensor::from(data))?;
            let prediction = self.adbanda.run(&embedding)?;

            assert_eq!(prediction.shape(), [1, 2].into());
            (prediction[0], prediction[1])
        };

        // Ignore Talk
        Ok(PredictedLabels::from_shape_vec(
            (1, 3),
            vec![ads, other, 0.0],
        )?)
    }
}

struct MoatClassifier {
    yamnet: TfModel,
    adbanda_mo: TfModel,
    adbanda_at: TfModel,
}

impl MoatClassifier {
    fn load<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
        Ok(Self {
            yamnet: TfModel::yamnet(&dir)?,
            adbanda_at: TfModel::adbanda(&dir, "adbdanda_at")?,
            adbanda_mo: TfModel::adbanda(&dir, "adbdanda_mo")?,
        })
    }
}

impl Classify for MoatClassifier {
    fn classify(&self, data: &Data) -> anyhow::Result<PredictedLabels> {
        let embedding: Tensor<f32> = self.yamnet.run(&Tensor::from(data))?;

        let (music, other) = {
            let prediction = self.adbanda_mo.run(&embedding)?;

            assert_eq!(prediction.shape(), [1, 2].into());
            (prediction[0], prediction[1])
        };

        let (ads, talk) = {
            let prediction = self.adbanda_at.run(&embedding)?;

            assert_eq!(prediction.shape(), [1, 2].into());
            (prediction[0], prediction[1])
        };

        Ok(PredictedLabels::from_shape_vec(
            (1, 3),
            vec![other * ads, music, other * talk],
        )?)
    }
}
