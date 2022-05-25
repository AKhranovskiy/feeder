use std::time::Duration;

use bytes::Bytes;
use reqwest::Url;

#[derive(Debug)]
pub struct Segment {
    pub info: SegmentInfo,
    pub content: Option<Bytes>,
    pub content_type: Option<String>,
    pub tags: Option<Vec<SegmentTags>>,
}

impl From<SegmentInfo> for Segment {
    fn from(info: SegmentInfo) -> Self {
        Self {
            info,
            content: None,
            content_type: None,
            tags: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SegmentTags {
    title: String,
    artist: String,
    other: String,
}

#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub url: Url,
    pub duration: Duration,
    pub title: Option<String>,
}
