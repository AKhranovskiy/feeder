use codec::{AudioFrame, Pts, Timestamp};

use super::Mixer;

pub struct PassthroughMixer(Pts);

impl PassthroughMixer {
    pub fn new() -> Self {
        Self(Pts::new(2048, 48_000))
    }
}

impl PassthroughMixer {
    fn pts(&mut self) -> Timestamp {
        self.0.next()
    }
}

impl Mixer<'_> for PassthroughMixer {
    fn content(&mut self, frame: AudioFrame) -> AudioFrame {
        frame.with_pts(self.pts())
    }

    fn advertisement(&mut self, frame: AudioFrame) -> AudioFrame {
        frame.with_pts(self.pts())
    }
}
