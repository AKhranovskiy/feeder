use codec::{AudioFrame, Pts, Timestamp};

use super::Mixer;

pub struct PassthroughMixer(Option<Pts>);

impl PassthroughMixer {
    pub fn new() -> Self {
        Self(None)
    }
}

impl PassthroughMixer {
    fn pts(&mut self, frame: &AudioFrame) -> Timestamp {
        if self.0.is_none() {
            self.0 = Some(Pts::from(frame));
        }

        self.0.as_mut().unwrap().next()
    }
}
impl Mixer for PassthroughMixer {
    fn push(&mut self, _kind: analyzer::ContentKind, frame: AudioFrame) -> AudioFrame {
        let pts = self.pts(&frame);
        frame.with_pts(pts)
    }
}
