#![feature(decl_macro)]

#[macro_use]
extern crate rocket;

use rocket::http::Status;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use rocket::Config;

use model::{Segment, SegmentInsertResponse, SegmentMatchResponse, SegmentUploadResponse};
use uuid::Uuid;

#[post("/segments/upload", format = "msgpack", data = "<segment>")]
async fn segments_upload(
    segment: MsgPack<Segment>,
) -> status::Custom<MsgPack<SegmentUploadResponse>> {
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

#[launch]
fn rocket() -> _ {
    let config = Config {
        port: 3456,
        ..Config::debug_default()
    };

    rocket::build()
        .configure(config)
        .mount("/api/v1", routes![segments_upload])
}
