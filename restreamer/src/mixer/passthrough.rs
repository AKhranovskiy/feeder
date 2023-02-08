use codec::AudioFrame;

use super::Mixer;

pub struct PassthroughMixer;

impl Mixer for PassthroughMixer {
    fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
        frame.clone()
    }

    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        frame.clone()
    }
}
