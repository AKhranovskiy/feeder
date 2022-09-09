#![allow(dead_code)] // Keep file as example of event stream.

use flume::Receiver;
use rocket::response::stream::{Event, EventStream};
use rocket::{get, Shutdown, State};
use serde::{Deserialize, Serialize};
use tokio::select;

use model::{SegmentInsertResponse, SegmentMatchResponse};

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
    events: &State<Receiver<FeederEvent>>,
    mut end: Shutdown,
) -> EventStream![] {
    let filter = EventFilter::from(filter);

    let events = events.inner().clone();

    EventStream! {
        loop {
            let ev = select! {
                Ok(ev) = events.recv_async() => ev,
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
