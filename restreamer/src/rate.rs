#![allow(dead_code)]

use std::time::Instant;

use ringbuf::{Rb, StaticRb};

pub struct Rate {
    queue: StaticRb<Item, 100>,
    clock: Instant,
}

struct Item(f32, usize);

impl Rate {
    pub fn new() -> Self {
        Self {
            queue: StaticRb::default(),
            clock: Instant::now(),
        }
    }

    pub fn push(&mut self, size: usize) -> usize {
        self.queue
            .push_overwrite(Item(self.clock.elapsed().as_secs_f32(), size));

        if let Some(first) = self.queue.iter().next() {
            let total = self.queue.iter().map(|item| item.1).sum::<usize>() as f32;
            let time = Instant::now().duration_since(self.clock).as_secs_f32() - first.0;
            (total / time).round() as usize
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate() {
        let mut rate = Rate::new();
        for _ in 0..100_000 {
            rate.push(1);
        }

        let r = rate.push(1);
        assert!(r > 100_000);
    }
}
