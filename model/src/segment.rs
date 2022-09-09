use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub url: String,
    pub duration: Duration,
    pub content: Vec<u8>,
    pub content_type: String,
    pub tags: Tags,
}

pub use tags::Tags;

impl Segment {
    #[deprecated]
    pub fn artist(&self) -> String {
        None.or_else(|| self.tags.track_artist())
            .or_else(|| self.tags.album_artist())
            .map(ToString::to_string)
            .unwrap_or_default()
    }

    #[deprecated]
    pub fn title(&self) -> String {
        self.tags
            .track_title()
            .map(ToString::to_string)
            .unwrap_or_default()
    }
}
