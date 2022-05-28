use std::collections::BTreeMap;
use std::time::Duration;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub url: Url,
    pub duration: Duration,
    pub content: Bytes,
    pub tags: Tags,
}

pub type Tags = BTreeMap<String, String>;

impl Segment {
    pub fn artist(&self) -> String {
        self.tags
            .get(&"TrackArtist".to_string())
            .or_else(|| self.tags.get(&"AlbumArtist".to_string()))
            .cloned()
            .unwrap_or_default()
    }
    pub fn title(&self) -> String {
        self.tags
            .get(&"TrackTitle".to_string())
            .cloned()
            .unwrap_or_default()
    }
}
