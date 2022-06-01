use anyhow::Context;
use bytes::Bytes;
use model::{ContentKind, Segment, SegmentMatchResponse, SegmentUploadResponse, Tags};
use mongodb::bson::{DateTime, Uuid};
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use rocket::tokio::sync::broadcast::Sender;
use rocket::State;
use rocket_db_pools::Connection;
use serde::{Deserialize, Serialize};

use crate::api::FeederEvent;
use crate::internal::emysound::{find_matches, insert_segment};
use crate::internal::Storage;

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
struct AudioDocument {
    id: Uuid,
    date_time: DateTime,
    kind: ContentKind,
    artist: String,
    title: String,
    content: Bytes,
    tags: Tags,
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

#[post("/segments/upload", format = "msgpack", data = "<segment>")]
pub async fn upload(
    segment: MsgPack<Segment>,
    storage: Connection<Storage>,
    events: &State<Sender<FeederEvent>>,
) -> Result<status::Custom<MsgPack<SegmentUploadResponse>>, status::Custom<String>> {
    log::info!("Uploaded segment: {} {}", segment.artist(), segment.title());

    if let Some(matches) = find_matches(&segment)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?
    {
        storage
            .database("feeder")
            .collection("matches")
            .insert_many(matches.iter().map(MatchDocument::from), None)
            .await
            .context("Registering matches")
            .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

        for m in &matches {
            let _res = events.send(m.clone().into());
        }

        return Ok(status::Custom(
            Status::Ok,
            SegmentUploadResponse::Matched(matches).into(),
        ));
    }

    let insert = insert_segment(&segment)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    storage
        .database("feeder")
        .collection("audio")
        .insert_one(
            AudioDocument::new(
                &*segment,
                Uuid::from_bytes(insert.id.into_bytes()),
                insert.kind,
            ),
            None,
        )
        .await
        .context("Registering new segment")
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    let _res = events.send(insert.clone().into());

    Ok(status::Custom(
        Status::Created,
        SegmentUploadResponse::Inserted(insert).into(),
    ))
}
