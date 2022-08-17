use model::ContentKind;
use serde::Serialize;

#[derive(Debug, Copy, Clone, Default, Serialize)]
pub struct Prediction {
    advertisement: f32,
    music: f32,
    talk: f32,
}

impl Prediction {
    pub fn new(advertisement: f32, music: f32, talk: f32) -> Self {
        Self {
            advertisement,
            music,
            talk,
        }
    }
}

impl From<[f32; 3]> for Prediction {
    fn from(values: [f32; 3]) -> Self {
        Self::new(values[0], values[1], values[2])
    }
}

impl From<ContentKind> for Prediction {
    fn from(kind: ContentKind) -> Self {
        match kind {
            ContentKind::Advertisement => [1.0, 0.0, 0.0].into(),
            ContentKind::Music => [0.0, 1.0, 0.0].into(),
            ContentKind::Talk => [0.0, 0.0, 1.0].into(),
            ContentKind::Unknown => Self::default(),
        }
    }
}

const PREDICTION_THRESHOLD: f32 = 0.65;

impl From<&Prediction> for ContentKind {
    fn from(prediction: &Prediction) -> Self {
        if prediction.advertisement >= PREDICTION_THRESHOLD {
            ContentKind::Advertisement
        } else if prediction.music >= PREDICTION_THRESHOLD {
            ContentKind::Music
        } else if prediction.talk >= PREDICTION_THRESHOLD {
            ContentKind::Talk
        } else {
            ContentKind::Unknown
        }
    }
}
