use model::{Segment, SegmentUploadResponse};

use rocket::http::Status;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use rocket::tokio::sync::broadcast::Sender;
use rocket::State;

use crate::api::FeederEvent;
use crate::internal::emysound::{find_matches, insert_segment};

#[post("/segments/upload", format = "msgpack", data = "<segment>")]
pub async fn upload(
    segment: MsgPack<Segment>,
    events: &State<Sender<FeederEvent>>,
) -> Result<status::Custom<MsgPack<SegmentUploadResponse>>, status::Custom<String>> {
    if let Some(matches) = find_matches(&segment)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?
    {
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

    let _res = events.send(insert.clone().into());

    Ok(status::Custom(
        Status::Created,
        SegmentUploadResponse::Inserted(insert).into(),
    ))
}
