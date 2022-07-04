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
    MatchedSegment(SegmentMatchResponse),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventFilter {
    All,
    New,
    Match,
}

impl FeederEvent {
    fn allowed(&self, filter: EventFilter) -> bool {
        match (self, filter) {
            (_, EventFilter::All) => true,
            (FeederEvent::NewSegment(_), EventFilter::New) => true,
            (FeederEvent::NewSegment(_), EventFilter::Match) => false,
            (FeederEvent::MatchedSegment(_), EventFilter::New) => false,
            (FeederEvent::MatchedSegment(_), EventFilter::Match) => true,
        }
    }
}

impl From<Option<&str>> for EventFilter {
    fn from(value: Option<&str>) -> Self {
        match value {
            Some("new") => Self::New,
            Some("match") => Self::Match,
            _ => Self::All,
        }
    }
}

/// Returns an infinite stream of server-sent events. Each event is a message
/// pulled from a broadcast queue sent by the `post` handler.
#[get("/events?<filter>")]
pub async fn events(
    filter: Option<&str>,
    events: &State<Sender<FeederEvent>>,
    mut end: Shutdown,
) -> EventStream![] {
    let filter = EventFilter::from(filter);

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

            if ev.allowed(filter) {
                yield Event::json(&ev)
            }
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
        FeederEvent::MatchedSegment(value)
    }
}
