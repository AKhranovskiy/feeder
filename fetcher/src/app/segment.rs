use std::collections::BTreeMap;
use std::time::Duration;

use bytes::Bytes;
use reqwest::Url;

#[derive(Debug)]
pub struct Segment {
    pub info: SegmentInfo,
    pub content: Option<Bytes>,
    pub content_type: Option<String>,
    pub tags: Tags,
}

impl From<SegmentInfo> for Segment {
    fn from(info: SegmentInfo) -> Self {
        Self {
            info,
            content: None,
            content_type: None,
            tags: BTreeMap::new(),
        }
    }
}

pub type Tags = BTreeMap<String, String>;

#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub url: Url,
    pub duration: Duration,
    pub title: Option<String>,
}
