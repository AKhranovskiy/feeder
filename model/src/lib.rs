mod segment;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use segment::{Segment, Tags};
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
