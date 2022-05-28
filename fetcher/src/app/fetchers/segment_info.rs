use std::time::Duration;

use bytes::Bytes;
use reqwest::Url;

use model::{Segment, Tags};

#[derive(Debug)]
pub struct SegmentInfo {
    pub url: Url,
    pub duration: Duration,
    pub title: Option<String>,
}

impl From<SegmentInfo> for Segment {
    fn from(info: SegmentInfo) -> Self {
        let mut tags = Tags::new();
        if let Some(title) = info.title {
            tags.insert("PlaylistTitle".to_string(), title);
        }
        tags.insert("AudioFileURL".to_string(), info.url.to_string());

        Self {
            url: info.url,
            duration: info.duration,
            content: Bytes::new(),
            tags,
        }
    }
}
