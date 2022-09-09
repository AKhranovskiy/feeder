mod segment;

use bson::Bson;
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
    Ignored,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContentKind {
    Advertisement,
    Music,
    Talk,
    Unknown,
}

impl TryFrom<&str> for ContentKind {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Advertisement" => Ok(ContentKind::Advertisement),
            "Music" => Ok(ContentKind::Music),
            "Talk" => Ok(ContentKind::Talk),
            "Unknown" => Ok(ContentKind::Unknown),
            _ => anyhow::bail!("Unknown content kind: {value}"),
        }
    }
}

impl ToString for ContentKind {
    fn to_string(&self) -> String {
        match self {
            ContentKind::Advertisement => "Advertisement",
            ContentKind::Music => "Music",
            ContentKind::Talk => "Talk",
            ContentKind::Unknown => "Unknown",
        }
        .to_owned()
    }
}

impl From<ContentKind> for Bson {
    fn from(kind: ContentKind) -> Self {
        kind.to_string().into()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataWithAudio {
    pub id: uuid::Uuid,
    pub kind: ContentKind,
    pub artist: String,
    pub title: String,
    pub r#type: String,
    pub content: Vec<u8>,
}
