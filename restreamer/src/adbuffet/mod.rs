#![allow(dead_code)]

use std::cell::Cell;
use std::fmt::Debug;
use std::path::Path;

mod entry;
pub use entry::AdEntry;

#[derive(Debug, Clone)]
pub struct AdBuffet {
    queue: Vec<AdEntry>,
    pos: Cell<usize>,
}

impl TryFrom<&[&Path]> for AdBuffet {
    type Error = anyhow::Error;

    fn try_from(paths: &[&Path]) -> Result<Self, Self::Error> {
        let queue = paths
            .iter()
            .map(|&path| AdEntry::try_from(path))
            .collect::<anyhow::Result<_>>()?;

        Ok(Self {
            queue,
            pos: Cell::default(),
        })
    }
}

impl AdBuffet {
    pub fn next(&self) -> Option<&AdEntry> {
        if self.queue.is_empty() {
            return None;
        }

        let pos = self.pos.get();
        assert!(pos < self.queue.len());
        self.pos.set((pos + 1) % self.queue.len());

        self.queue.get(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let sut = AdBuffet::empty();
        assert!(sut.next().is_none());
    }

    #[test]
    fn test_single() {
        let sut = AdBuffet::from(["single"]);
        assert_eq!(sut.next().map_or("", AdEntry::name), "single");
        assert_eq!(sut.next().map_or("", AdEntry::name), "single");
    }

    #[test]
    fn test_few() {
        let sut = AdBuffet::from(["first", "second", "third"]);
        assert_eq!(sut.next().map_or("", AdEntry::name), "first");
        assert_eq!(sut.next().map_or("", AdEntry::name), "second");
        assert_eq!(sut.next().map_or("", AdEntry::name), "third");
        assert_eq!(sut.next().map_or("", AdEntry::name), "first");
        assert_eq!(sut.next().map_or("", AdEntry::name), "second");
        assert_eq!(sut.next().map_or("", AdEntry::name), "third");
    }

    impl AdBuffet {
        fn empty() -> Self {
            Self {
                queue: Vec::default(),
                pos: Cell::default(),
            }
        }
    }

    impl<const N: usize> From<[&str; N]> for AdBuffet {
        fn from(names: [&str; N]) -> Self {
            Self {
                queue: names.into_iter().map(AdEntry::from_name).collect(),
                pos: Cell::default(),
            }
        }
    }
}
