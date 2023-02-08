use codec::AudioFrame;

use super::Mixer;

pub struct PassthroughMixer;

impl Mixer for PassthroughMixer {
    #[inline(always)]
    fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
        frame.clone()
    }

    #[inline(always)]
    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        frame.clone()
    }
}
