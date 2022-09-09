use model::ContentKind;
use serde::Serialize;

#[derive(Debug, Copy, Clone, Default, Serialize)]
pub struct Prediction {
    advertisement: f32,
    music: f32,
    talk: f32,
}

impl Prediction {
    pub fn get(&self, kind: &ContentKind) -> f32 {
        match kind {
            ContentKind::Advertisement => self.advertisement,
            ContentKind::Music => self.music,
            ContentKind::Talk => self.talk,
            ContentKind::Unknown => 1.0f32,
        }
    }

    pub fn max(&self) -> (ContentKind, f32) {
        [
            (ContentKind::Advertisement, self.advertisement),
            (ContentKind::Music, self.music),
            (ContentKind::Talk, self.talk),
        ]
        .iter()
        .max_by_key(|(_, v)| (v * 100f32).round() as u32)
        .cloned()
        .unwrap_or((ContentKind::Unknown, 1.0f32))
    }
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

impl From<&Prediction> for ContentKind {
    fn from(prediction: &Prediction) -> Self {
        [
            (ContentKind::Advertisement, prediction.advertisement),
            (ContentKind::Music, prediction.music),
            (ContentKind::Talk, prediction.talk),
        ]
        .iter()
        .max_by_key(|(_, v)| (v * 100f32).round() as u32)
        .map(|(kind, _)| *kind)
        .unwrap_or_else(|| ContentKind::Unknown)
        // TODO - normal mapping would accounf for threshold.
        // However, there is no reliable classification yet, so give whatever is maximum,
        //
        // const PREDICTION_THRESHOLD: f32 = 0.65;
        // if prediction.advertisement >= PREDICTION_THRESHOLD {
        //     ContentKind::Advertisement
        // } else if prediction.music >= PREDICTION_THRESHOLD {
        //     ContentKind::Music
        // } else if prediction.talk >= PREDICTION_THRESHOLD {
        //     ContentKind::Talk
        // } else {
        //     ContentKind::Unknown
        // }
    }
}
