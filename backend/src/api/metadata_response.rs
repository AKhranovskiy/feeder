use model::{ContentKind, Tags};
use serde::{Deserialize, Serialize};

use crate::internal::storage::MetadataDocument;

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataResponse {
    pub id: uuid::Uuid,
    pub date_time: chrono::DateTime<chrono::Utc>,
    pub kind: ContentKind,
    pub artist: String,
    pub title: String,
    pub tags: Tags,
}

impl From<&MetadataDocument> for MetadataResponse {
    fn from(doc: &MetadataDocument) -> Self {
        Self {
            id: uuid::Uuid::from_bytes(doc.id.bytes()),
            date_time: doc.date_time.to_chrono(),
            kind: doc.kind,
            artist: doc.artist.clone(),
            tags: doc.tags.clone(),
            title: doc.title.clone(),
        }
    }
}
