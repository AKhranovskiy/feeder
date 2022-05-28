use model::{Segment, SegmentInsertResponse, SegmentMatchResponse, SegmentUploadResponse};

use rocket::http::Status;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use rocket::tokio::sync::broadcast::Sender;
use rocket::State;
use uuid::Uuid;

use crate::api::FeederEvent;

#[post("/segments/upload", format = "msgpack", data = "<segment>")]
pub async fn upload(
    segment: MsgPack<Segment>,
    events: &State<Sender<FeederEvent>>,
) -> status::Custom<MsgPack<SegmentUploadResponse>> {
    if segment.content.len() % 2 == 0 {
        let data = SegmentInsertResponse {
            id: Uuid::new_v4(),
            artist: "Artist".to_string(),
            title: "Title".to_string(),
            kind: "Music".to_string(),
        };

        // Ignore failures;
        let _res = events.send(data.clone().into());

        status::Custom(
            Status::Created,
            SegmentUploadResponse::Inserted(data).into(),
        )
    } else {
        let matches = vec![
            SegmentMatchResponse {
                id: Uuid::new_v4(),
                score: 128,
                artist: "Artist".to_string(),
                title: "Title".to_string(),
                kind: "Music".to_string(),
            },
            SegmentMatchResponse {
                id: Uuid::new_v4(),
                score: 250,
                artist: "Artist".to_string(),
                title: "Title".to_string(),
                kind: "Music".to_string(),
            },
        ];
        for m in &matches {
            let _res = events.send(m.clone().into());
        }

        status::Custom(Status::Ok, SegmentUploadResponse::Matched(matches).into())
    }
}
