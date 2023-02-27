use std::time::Duration;

use ac_ffmpeg::codec::audio::AudioFrame;
use ac_ffmpeg::time::Timestamp;

use crate::FrameDuration;

pub struct Pts {
    pts: Timestamp,
    duration: Duration,
}

impl From<&AudioFrame> for Pts {
    fn from(frame: &AudioFrame) -> Self {
        let pts = Timestamp::new(0, frame.time_base());
        let duration = frame.duration();
        Self { pts, duration }
    }
}

impl Pts {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Timestamp {
        let pts = self.pts;
        self.pts += self.duration;
        pts
    }
}

#[cfg(test)]
mod tests {
    use ac_ffmpeg::codec::audio::{AudioFrameMut, ChannelLayout};

    use crate::{Pts, SampleFormat};

    #[test]
    fn test_1024_of_44_100() {
        let frame = AudioFrameMut::silence(
            ChannelLayout::from_channels(1).unwrap().as_ref(),
            SampleFormat::Flt.into(),
            44_100,
            1024,
        )
        .freeze();

        let mut pts = Pts::from(&frame);
        assert_eq!(pts.next().as_nanos(), Some(0));
        assert_eq!(pts.next().as_nanos(), Some(23_219_000));
        assert_eq!(pts.next().as_nanos(), Some(46_438_000));
    }

    #[test]
    fn test_4_of_4() {
        let frame = AudioFrameMut::silence(
            ChannelLayout::from_channels(1).unwrap().as_ref(),
            SampleFormat::Flt.into(),
            4,
            4,
        )
        .freeze();

        let mut pts = Pts::from(&frame);
        assert_eq!(pts.next().as_nanos(), Some(0));
        assert_eq!(pts.next().as_nanos(), Some(1_000_000_000));
        assert_eq!(pts.next().as_nanos(), Some(2_000_000_000));
    }

    #[test]
    fn test_960_of_48_000() {
        let frame = AudioFrameMut::silence(
            ChannelLayout::from_channels(2).unwrap().as_ref(),
            SampleFormat::Flt.into(),
            48_000,
            960,
        )
        .freeze();

        let mut pts = Pts::from(&frame);
        assert_eq!(pts.next().as_nanos(), Some(0));
        assert_eq!(pts.next().as_nanos(), Some(20_000_000));
        assert_eq!(pts.next().as_nanos(), Some(40_000_000));
    }
}
