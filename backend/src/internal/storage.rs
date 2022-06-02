use anyhow::Context;
use bytes::Bytes;
use model::{ContentKind, Segment, SegmentInsertResponse, SegmentMatchResponse, Tags};
use mongodb::bson::{DateTime, Uuid};
use rocket_db_pools::mongodb::Client;
use rocket_db_pools::{Connection, Database};
use serde::{Deserialize, Serialize};

#[derive(Database)]
#[database("storage")]
pub struct Storage(Client);

#[derive(Debug, Serialize, Deserialize)]
struct MatchDocument {
    id: Uuid,
    date_time: DateTime,
    score: u8,
}

impl From<&SegmentMatchResponse> for MatchDocument {
    fn from(value: &SegmentMatchResponse) -> Self {
        MatchDocument {
            id: Uuid::from_bytes(value.id.into_bytes()),
            date_time: DateTime::now(),
            score: value.score,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioDocument {
    pub id: Uuid,
    pub date_time: DateTime,
    pub kind: ContentKind,
    pub artist: String,
    pub title: String,
    pub content: Bytes,
    pub tags: Tags,
}

impl AudioDocument {
    fn new(segment: &Segment, id: Uuid, kind: ContentKind) -> Self {
        Self {
            id,
            date_time: DateTime::now(),
            kind,
            artist: segment.artist(),
            title: segment.title(),
            content: segment.content.clone(),
            tags: segment.tags.clone(),
        }
    }
}

pub async fn insert_matches(
    conn: &Connection<Storage>,
    matches: &[SegmentMatchResponse],
) -> anyhow::Result<()> {
    conn.database("feeder")
        .collection("matches")
        .insert_many(matches.iter().map(MatchDocument::from), None)
        .await
        .context("Insert matches")
        .map(|_| ())
}

pub async fn insert_audio(
    conn: &Connection<Storage>,
    segment: &Segment,
    response: &SegmentInsertResponse,
) -> anyhow::Result<()> {
    conn.database("feeder")
        .collection("audio")
        .insert_one(
            AudioDocument::new(
                segment,
                Uuid::from_bytes(response.id.into_bytes()),
                response.kind,
            ),
            None,
        )
        .await
        .context("Registering new segment")
        .map(|_| ())
}
