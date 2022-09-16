use rocket::get;
use rocket::http::ContentType;
use rocket::serde::json::Json;
use rocket_db_pools::Connection;

use crate::internal::storage::Storage;
use crate::storage::playback::Playback;
use crate::storage::StorageScheme;

#[get("/playbacks")]
pub async fn all_segments(_storage: Connection<Storage>) -> Option<Json<Vec<String>>> {
    None
}

#[get("/playbacks/stream/<_stream>")]
pub async fn segments_for_stream(
    _storage: Connection<Storage>,
    _stream: &str,
) -> Option<Json<Vec<String>>> {
    None
}

#[get("/playbacks/segment/<segment>")]
pub async fn one_segment(
    storage: Connection<Storage>,
    segment: &str,
) -> Option<(ContentType, Vec<u8>)> {
    storage.playbacks().get(segment).await.ok()?.map(prepare)
}

fn prepare(playback: Playback) -> (ContentType, Vec<u8>) {
    let content_type =
        ContentType::parse_flexible(&playback.content_type).unwrap_or(ContentType::Binary);
    (content_type, playback.content)
}
