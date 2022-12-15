use std::collections::BTreeMap;

use anyhow::Context;
use model::{ContentKind, Segment, SegmentMatchResponse};
use mongodb::bson::{DateTime, Uuid};
use rocket_db_pools::mongodb::Client;
use rocket_db_pools::{Connection, Database};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, Bytes};

#[derive(Database)]
#[database("storage")]
pub struct Storage(Client);

#[derive(Debug, Serialize, Deserialize)]
pub struct MatchDocument {
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

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct AudioDocument {
    pub id: Uuid,
    #[serde_as(as = "Bytes")]
    pub content: Vec<u8>,
    pub r#type: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataDocument {
    pub id: Uuid,
    pub date_time: DateTime,
    pub kind: ContentKind,
    pub artist: String,
    pub title: String,
    // Must be BTreeMap because it is stored in DB.
    // Changing type would require wiping all records.
    pub tags: BTreeMap<String, String>,
}

impl MetadataDocument {
    fn new(segment: &Segment, id: Uuid, kind: ContentKind) -> Self {
        Self {
            id,
            date_time: DateTime::now(),
            kind,
            artist: segment.tags.track_artist_or_empty(),
            title: segment.tags.track_title_or_empty(),
            tags: segment.tags.clone().into(),
        }
    }
}

pub async fn add_segment(
    conn: &Connection<Storage>,
    segment: &Segment,
    id: uuid::Uuid,
    kind: ContentKind,
) -> anyhow::Result<()> {
    let id = Uuid::from_bytes(id.into_bytes());

    conn.database("feeder")
        .collection("audio")
        .insert_one(
            AudioDocument {
                id,
                content: segment.content.clone(),
                r#type: segment.content_type.clone(),
            },
            None,
        )
        .await
        .context("Insert audio data")
        .map(|_| ())?;

    conn.database("feeder")
        .collection("metadata")
        .insert_one(MetadataDocument::new(segment, id, kind), None)
        .await
        .context("Insert metadata")
        .map(|_| ())?;

    Ok(())
}
