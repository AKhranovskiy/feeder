use std::sync::atomic::Ordering;

use codec::AudioFrame;

use super::events::PlayerId;
use super::listener::PlayEventListener;
use super::AdEntry;

pub struct AdEntryFrameIterator<'entry> {
    entry: &'entry AdEntry,
    frames: &'entry [AudioFrame],
    pos: usize,
    player_id: PlayerId,
    listener: &'entry PlayEventListener,
}

impl<'entry> Iterator for AdEntryFrameIterator<'entry> {
    type Item = &'entry AudioFrame;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.pos;

        if pos < self.frames.len() {
            self.pos += 1;
            self.listener.notify(self.player_id, self.pos);
            self.frames.get(pos)
        } else {
            None
        }
    }
}

impl<'entry> IntoIterator for &'entry AdEntry {
    type Item = &'entry AudioFrame;

    type IntoIter = AdEntryFrameIterator<'entry>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            entry: self,
            frames: &self.frames,
            pos: 0_usize,
            player_id: self.next_player_id.fetch_add(1, Ordering::AcqRel),
            listener: &self.event_listener,
        }
    }
}
