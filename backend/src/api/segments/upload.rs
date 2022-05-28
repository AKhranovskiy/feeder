use model::{Segment, SegmentInsertResponse, SegmentMatchResponse, SegmentUploadResponse};

use rocket::http::Status;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use uuid::Uuid;

#[post("/segments/upload", format = "msgpack", data = "<segment>")]
pub async fn upload(segment: MsgPack<Segment>) -> status::Custom<MsgPack<SegmentUploadResponse>> {
    if segment.content.len() % 2 == 0 {
        status::Custom(
            Status::Created,
            SegmentUploadResponse::Inserted(SegmentInsertResponse {
                id: Uuid::new_v4(),
                artist: "Artist".to_string(),
                title: "Title".to_string(),
                kind: "Music".to_string(),
            })
            .into(),
        )
    } else {
        status::Custom(
            Status::Ok,
            SegmentUploadResponse::Matched(vec![
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
            ])
            .into(),
        )
    }
}
