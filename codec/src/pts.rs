use std::time::Duration;

use ac_ffmpeg::{codec::audio::AudioFrame, time::Timestamp};

use crate::FrameDuration;

pub struct Pts {
    duration: Duration,
    counter: u32,
}

impl Pts {
    #[must_use]
    pub fn new(samples_per_frame: u32, sample_rate: u32) -> Pts {
        let duration = f64::from(samples_per_frame) / f64::from(sample_rate);
        let duration = Duration::from_secs_f64(duration);
        let counter = 0;

        Self { duration, counter }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Timestamp {
        let timestamp = self.duration * self.counter;
        self.counter += 1;
        Timestamp::from_micros(timestamp.as_micros().try_into().unwrap_or_default())
    }

    pub fn update(&mut self, frame: &AudioFrame) {
        self.duration = frame.duration();
    }
}

#[cfg(test)]
mod tests {
    use crate::Pts;

    #[test]
    fn test_1024_of_44_100() {
        let mut pts = Pts::new(1_024, 44_100);

        assert_eq!(pts.next().as_micros(), Some(0));
        assert_eq!(pts.next().as_micros(), Some(23_219));
        assert_eq!(pts.next().as_micros(), Some(46_439));
    }

    #[test]
    fn test_4_of_4() {
        let mut pts = Pts::new(4, 4);
        assert_eq!(pts.next().as_micros(), Some(0));
        assert_eq!(pts.next().as_micros(), Some(1_000_000));
        assert_eq!(pts.next().as_micros(), Some(2_000_000));
    }

    #[test]
    fn test_2048_of_48_000() {
        let mut pts = Pts::new(2_048, 48_000);
        assert_eq!(pts.next().as_micros(), Some(0));
        assert_eq!(pts.next().as_micros(), Some(42_666));
        assert_eq!(pts.next().as_micros(), Some(85_333));
    }
}
