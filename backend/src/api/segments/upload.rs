use model::{ContentKind, Segment, SegmentMatchResponse, SegmentUploadResponse};
use mongodb::bson::doc;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use rocket::tokio::sync::broadcast::Sender;
use rocket::State;
use rocket_db_pools::Connection;

use crate::api::segments::to_internal_server_error;
use crate::api::FeederEvent;
use crate::internal::emysound::{find_matches, insert_segment};
use crate::internal::guess_content_kind;
use crate::internal::storage::{self, MetadataDocument, Storage};

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
        storage::insert_matches(&storage, &matches)
            .await
            .map_err(to_internal_server_error)?;

        for m in &matches {
            let _res = events.send(m.clone().into());
        }

        let mut matches2: Vec<SegmentMatchResponse> = Vec::new();
        for m in matches.into_iter() {
            let kind = retrieve_content_kind(&storage, &m.id).await;
            matches2.push(SegmentMatchResponse { kind, ..m });
        }

        return Ok(status::Custom(
            Status::Ok,
            SegmentUploadResponse::Matched(matches2).into(),
        ));
    }

    let kind = guess_content_kind(&segment.tags);

    let response = insert_segment(&segment, kind)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    storage::insert_audio(&storage, &segment, response.id, kind)
        .await
        .map_err(to_internal_server_error)?;

    let _res = events.send(response.clone().into());

    Ok(status::Custom(
        Status::Created,
        SegmentUploadResponse::Inserted(response).into(),
    ))
}

async fn retrieve_content_kind(storage: &Connection<Storage>, id: &uuid::Uuid) -> ContentKind {
    let id = mongodb::bson::Uuid::from_bytes(id.into_bytes());
    storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata")
        .find_one(doc! {"id": id}, None)
        .await
        .ok()
        .flatten()
        .map(|doc| doc.kind)
        .unwrap_or(ContentKind::Unknown)
}
