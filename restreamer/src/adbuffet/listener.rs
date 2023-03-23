use std::sync::{Arc, Mutex};

use time::macros::offset;

use super::events::{PlayEvent, PlayerId};

#[derive(Clone)]
pub struct PlayEventListener {
    total: usize,
    events: Arc<Mutex<Vec<PlayEvent>>>,
}

impl PlayEventListener {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            events: Arc::default(),
        }
    }

    pub fn notify(&self, player_id: PlayerId, position: usize) {
        let quarter = self.total / 4;

        let time = time::OffsetDateTime::now_utc().to_offset(offset!(+7));

        if let Some(event) = if position == 1 {
            Some(PlayEvent::Start(player_id, time))
        } else if position == quarter {
            Some(PlayEvent::FirstQuarter(player_id, time))
        } else if position == (quarter * 2) {
            Some(PlayEvent::Median(player_id, time))
        } else if position == (quarter * 3) {
            Some(PlayEvent::ThirdQuarter(player_id, time))
        } else if position == self.total {
            Some(PlayEvent::End(player_id, time))
        } else {
            None
        } {
            self.events.lock().unwrap().push(event);
        }
    }

    pub fn events(&self) -> Vec<PlayEvent> {
        self.events.lock().unwrap().clone()
    }
}
