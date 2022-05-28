use model::{
    ContentKind, Segment, SegmentInsertResponse, SegmentMatchResponse, SegmentUploadResponse,
};

use rocket::http::Status;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use rocket::tokio::sync::broadcast::Sender;
use rocket::State;
use uuid::Uuid;

use crate::api::FeederEvent;
use crate::internal;
use crate::internal::emysound::TrackInfo;

#[post("/segments/upload", format = "msgpack", data = "<segment>")]
pub async fn upload(
    segment: MsgPack<Segment>,
    events: &State<Sender<FeederEvent>>,
) -> Result<status::Custom<MsgPack<SegmentUploadResponse>>, status::Custom<String>> {
    let filename = segment.url.path();

    let matches: Vec<SegmentMatchResponse> = internal::emysound::query(filename, &segment.content)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?
        .iter()
        .map(|m| m.into())
        .collect();

    for m in &matches {
        let _res = events.send(m.clone().into());
    }

    if !matches.is_empty() {
        return Ok(status::Custom(
            Status::Ok,
            SegmentUploadResponse::Matched(matches).into(),
        ));
    }

    let artist = segment
        .tags
        .get(&"TrackArtist".to_string())
        .cloned()
        .unwrap_or_default();

    let title = segment
        .tags
        .get(&"TrackTitle".to_string())
        .cloned()
        .unwrap_or_default();

    let track_info = TrackInfo::new(Uuid::new_v4(), artist, title);

    internal::emysound::insert(track_info.clone(), filename, &segment.content)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    let response: SegmentInsertResponse = track_info.into();

    let _res = events.send(response.clone().into());

    Ok(status::Custom(
        Status::Created,
        SegmentUploadResponse::Inserted(response).into(),
    ))
}

impl From<TrackInfo> for SegmentInsertResponse {
    fn from(info: TrackInfo) -> Self {
        SegmentInsertResponse {
            id: info.id,
            artist: info.artist,
            title: info.title,
            kind: ContentKind::Unknown,
        }
    }
}

impl From<&internal::emysound::QueryResult> for SegmentMatchResponse {
    fn from(result: &internal::emysound::QueryResult) -> Self {
        SegmentMatchResponse {
            id: result.id,
            score: result.coverage,
            artist: result.artist.clone(),
            title: result.title.clone(),
            kind: ContentKind::Unknown,
        }
    }
}
