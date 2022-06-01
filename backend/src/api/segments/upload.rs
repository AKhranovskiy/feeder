use model::{Segment, SegmentUploadResponse};
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use rocket::tokio::sync::broadcast::Sender;
use rocket::State;
use rocket_db_pools::Connection;

use crate::api::segments::to_internal_server_error;
use crate::api::FeederEvent;
use crate::internal::emysound::{find_matches, insert_segment};
use crate::internal::storage::{self, Storage};

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

        return Ok(status::Custom(
            Status::Ok,
            SegmentUploadResponse::Matched(matches).into(),
        ));
    }

    let response = insert_segment(&segment)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    storage::insert_audio(&storage, &segment, &response)
        .await
        .map_err(to_internal_server_error)?;

    let _res = events.send(response.clone().into());

    Ok(status::Custom(
        Status::Created,
        SegmentUploadResponse::Inserted(response).into(),
    ))
}
