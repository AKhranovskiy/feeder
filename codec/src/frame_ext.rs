use std::time::Duration;

use ac_ffmpeg::codec::audio::AudioFrame;

pub trait FrameDuration {
    fn duration(&self) -> Duration;
}

impl FrameDuration for AudioFrame {
    fn duration(&self) -> Duration {
        let samples = self.samples() as f64;
        let rate = self.sample_rate() as f64;
        let channels = self.channel_layout().channels() as f64;
        Duration::from_secs_f64(samples / channels / rate)
    }
}

#[cfg(test)]
mod tests {
    use ac_ffmpeg::codec::audio::{AudioFrameMut, ChannelLayout};

    use crate::frame_ext::FrameDuration;
    use crate::SampleFormat;

    #[test]
    fn test_duration() {
        let frame = AudioFrameMut::silence(
            &ChannelLayout::from_channels(2).unwrap(),
            SampleFormat::Flt.into(),
            44100,
            1000,
        )
        .freeze();

        assert_eq!(frame.duration().as_millis(), 11);
    }
}