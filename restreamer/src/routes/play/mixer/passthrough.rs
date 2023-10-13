use axum::async_trait;
use codec::{AudioFrame, Pts};

use super::Mixer;

pub struct PassthroughMixer(Pts);

impl PassthroughMixer {
    pub fn new() -> Self {
        Self(Pts::new(2_048, 48_000))
    }
}

#[async_trait]
impl Mixer for PassthroughMixer {
    async fn push(&mut self, _kind: analyzer::ContentKind, frame: &AudioFrame) -> AudioFrame {
        self.0.update(frame);
        frame.clone().with_pts(self.0.next())
    }
}
