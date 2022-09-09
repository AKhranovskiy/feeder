use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use url::Url;

pub type StreamId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDocument {
    #[serde(rename = "_id")]
    id: ObjectId,
    pub name: String,
    pub url: Url,
}

impl StreamDocument {
    pub fn id(&self) -> StreamId {
        self.id.to_hex()
    }
}
