use futures::StreamExt;
use rocket::response::stream::{Event, EventStream};
use rocket::{get, Shutdown};
use rocket_db_pools::Connection;
use serde::Serialize;

use crate::internal::storage::Storage;
use crate::storage::playback::{Playback, PlaybackWatchEvent, WatchEvent};
use crate::storage::StorageScheme;

#[get("/playbacks/updates")]
pub async fn updates(storage: Connection<Storage>, stop: Shutdown) -> Option<EventStream![]> {
    storage.playbacks().watch(stop).await.map_or_else(
        |error| {
            log::error!("{error}");
            None
        },
        |stream| Some(stream.map(|ev| ev.into()).into()),
    )
}

impl From<PlaybackWatchEvent> for Event {
    fn from(ev: PlaybackWatchEvent) -> Self {
        match ev {
            WatchEvent::Add(id, playback) => Event::json(&AddPlaybackEvent::from(playback))
                .id(id)
                .event("add"),
            WatchEvent::Delete(id) => Event::empty().id(id).event("delete"),
            WatchEvent::Error(error) => Event::data(error.to_string()).event("error"),
        }
    }
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct AddPlaybackEvent {
    id: String,
    stream_id: String,
    url: String,
    duration_ms: u32,
    artist: String,
    title: String,
    classification: Vec<(String, f32)>,
}

impl From<Playback> for AddPlaybackEvent {
    fn from(playback: Playback) -> Self {
        let url = format!("/api/v1/playbacks/segment/{}", playback.id);
        Self {
            id: playback.id,
            stream_id: playback.stream_id,
            url,
            duration_ms: playback.duration.as_millis() as u32,
            artist: playback.artist,
            title: playback.title,
            classification: playback
                .classification
                .into_iter()
                .map(|(kind, conf)| (kind.to_string(), conf))
                .collect(),
        }
    }
}
