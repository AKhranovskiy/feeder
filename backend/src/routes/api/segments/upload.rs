use mongodb::bson::doc;
use rocket::http::Status;
use rocket::post;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use rocket_db_pools::Connection;

use model::{ContentKind, Segment, SegmentMatchResponse, SegmentUploadResponse};

use crate::internal::emysound::{add_fingerprints, find_matches};
use crate::internal::storage::{self, MetadataDocument, Storage};

use super::to_internal_server_error;

// TODO - remove
#[post("/segments/upload", format = "msgpack", data = "<segment>")]
pub async fn upload(
    segment: MsgPack<Segment>,
    storage: Connection<Storage>,
) -> Result<status::Custom<MsgPack<SegmentUploadResponse>>, status::Custom<String>> {
    if let Some(matches) = find_matches(&segment)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?
    {
        // TODO - Compare segment's info to matches info.
        storage::insert_matches(&storage, &matches)
            .await
            .map_err(to_internal_server_error)?;

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

    let kind = tag_analyser::analyse_tags(&segment.tags);

    // if kind != ContentKind::Unknown {
    let response = add_fingerprints(&segment, kind)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    storage::add_segment(&storage, &segment, response.id, kind)
        .await
        .map_err(to_internal_server_error)?;

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
