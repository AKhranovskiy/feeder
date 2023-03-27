use std::sync::Arc;

use codec::AudioFrame;

use crate::adbuffet::{AdBuffet, AdEntry, AdEntryFrameIterator};

pub struct AdsProvider<'ad> {
    buffet: &'ad AdBuffet,
    current_entry: Option<Arc<AdEntry>>,
    ad_iter: Option<AdEntryFrameIterator<'ad>>,
    total: usize,
    played: usize,
}

impl<'ad> AdsProvider<'ad> {
    pub fn new(buffet: &'ad AdBuffet) -> Self {
        Self {
            buffet,
            current_entry: None,
            ad_iter: None,
            total: 0,
            played: 0,
        }
    }

    pub fn next(&mut self) -> Option<&'ad AudioFrame> {
        let frame = self.ad_iter.as_mut().and_then(|x| x.next());
        if frame.is_some() {
            self.played += 1;
        }
        frame
    }

    pub fn remains(&self) -> usize {
        self.total - self.played
    }

    pub fn start<'a: 'ad>(&'a mut self) {
        self.current_entry = self.buffet.next();
        self.ad_iter = self.current_entry.as_ref().map(|x| x.into_iter());

        self.total = self
            .current_entry
            .as_deref()
            .map(|x| x.num_of_frames())
            .unwrap_or_default();
        self.played = 0;
    }

    pub fn len(&self) -> usize {
        self.total
    }
}
