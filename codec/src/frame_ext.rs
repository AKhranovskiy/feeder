use std::time::Duration;

use ac_ffmpeg::codec::audio::AudioFrame;

pub trait FrameDuration {
    fn duration(&self) -> Duration;
}

impl FrameDuration for AudioFrame {
    fn duration(&self) -> Duration {
        let samples_per_channel = self.samples() as f64;
        let rate = f64::from(self.sample_rate());
        Duration::from_secs_f64(samples_per_channel / rate)
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
            2048,
        )
        .freeze();

        assert_eq!(frame.duration().as_millis(), 46);
    }
}
