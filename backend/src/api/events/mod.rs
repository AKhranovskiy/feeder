use model::{SegmentInsertResponse, SegmentMatchResponse};
use rocket::response::stream::{Event, EventStream};
use rocket::tokio::select;
use rocket::tokio::sync::broadcast::{error::RecvError, Sender};
use rocket::{Shutdown, State};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum FeederEvent {
    NewSegment(SegmentInsertResponse),
    Match(SegmentMatchResponse),
}

/// Returns an infinite stream of server-sent events. Each event is a message
/// pulled from a broadcast queue sent by the `post` handler.
#[get("/events")]
pub async fn events(events: &State<Sender<FeederEvent>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = events.subscribe();
    EventStream! {
        loop {
            let ev = select! {
                ev = rx.recv() => match ev {
                    Ok(ev) => ev,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };


            yield Event::json(&ev).event(match ev {
                FeederEvent::NewSegment(_) => "new-segment",
                FeederEvent::Match(_) => "match"
            });
        }
    }
}

impl From<SegmentInsertResponse> for FeederEvent {
    fn from(value: SegmentInsertResponse) -> Self {
        FeederEvent::NewSegment(value)
    }
}

impl From<SegmentMatchResponse> for FeederEvent {
    fn from(value: SegmentMatchResponse) -> Self {
        FeederEvent::Match(value)
    }
}
