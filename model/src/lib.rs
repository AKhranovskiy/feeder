use std::collections::BTreeMap;
use std::time::Duration;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub url: Url,
    pub duration: Duration,
    pub content: Bytes,
    pub tags: Tags,
}

pub type Tags = BTreeMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMatchResponse {
    pub id: Uuid,
    pub score: u8,
    pub artist: String,
    pub title: String,
    pub kind: ContentKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentInsertResponse {
    pub id: Uuid,
    pub artist: String,
    pub title: String,
    pub kind: ContentKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegmentUploadResponse {
    Matched(Vec<SegmentMatchResponse>),
    Inserted(SegmentInsertResponse),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ContentKind {
    Unknown,
    Advertisement,
    Music,
    Talk,
}
