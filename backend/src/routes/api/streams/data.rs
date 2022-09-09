use serde::Serialize;

use crate::storage::streams::StreamDocument;

#[derive(Debug, Clone, Serialize)]
pub struct StreamData {
    pub id: String,
    pub name: String,
    pub url: String,
}

impl From<StreamDocument> for StreamData {
    fn from(doc: StreamDocument) -> Self {
        Self {
            id: doc.id(),
            name: doc.name,
            url: doc.url.to_string(),
        }
    }
}
