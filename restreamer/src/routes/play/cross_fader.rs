use std::cell::Cell;
use std::time::Duration;

use codec::dsp::{CrossFade, CrossFadePair};
use codec::AudioFrame;

pub struct CrossFader {
    values: Vec<CrossFadePair>,
    pos: Cell<usize>,
}

impl CrossFader {
    pub fn new<CF: CrossFade>(cf_duration: Duration, frame_duration: Duration) -> Self {
        let values = CF::generate((cf_duration.as_millis() / frame_duration.as_millis()) as usize);

        eprintln!(
            "Cross-fade {:0.1}s, {} frames",
            cf_duration.as_secs_f32(),
            values.len()
        );

        Self {
            values,
            pos: Cell::default(),
        }
    }

    #[allow(dead_code)]
    pub fn exact<CF: CrossFade>(length: usize) -> Self {
        let values = CF::generate(length);
        Self {
            values,
            pos: Cell::default(),
        }
    }

    pub fn reset(&self) {
        self.pos.set(0);
    }

    pub fn apply(&self, fade_out: &AudioFrame, fade_in: &AudioFrame) -> AudioFrame {
        let pos = self.pos.get();

        self.values.get(pos).map_or_else(
            || fade_in.clone(),
            |cf| {
                self.pos.set(pos + 1);
                cf * (fade_out, fade_in)
            },
        )
    }
}
