use std::time::{Duration, Instant};

use ringbuf::{Rb, StaticRb};

pub(crate) struct Rate {
    values: StaticRb<Duration, 100>,
    timer: Instant,
}

impl Rate {
    pub fn new() -> Self {
        Self {
            values: StaticRb::default(),
            timer: Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.timer = Instant::now();
    }

    pub fn stop(&mut self) {
        self.values.push_overwrite(self.timer.elapsed());
    }

    pub fn average(&self) -> Duration {
        match self.values.len() {
            0 => Duration::default(),
            len => Duration::from_millis(
                (self.values.iter().map(Duration::as_millis).sum::<u128>() / len as u128) as u64,
            ),
        }
    }
}

impl PartialOrd<Duration> for Rate {
    fn partial_cmp(&self, other: &Duration) -> Option<std::cmp::Ordering> {
        self.average().partial_cmp(other)
    }
}

impl PartialEq<Duration> for Rate {
    fn eq(&self, other: &Duration) -> bool {
        self.partial_cmp(other) == Some(std::cmp::Ordering::Equal)
    }
}
