use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataWithAudioDocument {
    pub id: mongodb::bson::Uuid,
    pub kind: model::ContentKind,
    pub artist: String,
    pub title: String,
    pub audio: Vec<super::storage::AudioDocument>,
}

impl From<MetadataWithAudioDocument> for model::MetadataWithAudio {
    fn from(mut doc: MetadataWithAudioDocument) -> Self {
        let audio = doc.audio.pop().unwrap();
        Self {
            id: uuid::Uuid::from_bytes(doc.id.bytes()),
            kind: doc.kind,
            artist: doc.artist,
            title: doc.title,
            r#type: audio.r#type,
            content: audio.content,
        }
    }
}
