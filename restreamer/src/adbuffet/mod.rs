#![allow(dead_code)]

use std::fmt::Debug;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

mod entry;
pub use entry::AdEntry;

use self::entry::AdEntryRef;

#[derive(Debug, Clone)]
pub struct AdBuffet {
    queue: Vec<Arc<AdEntry>>,
    pos: Arc<AtomicUsize>,
}

impl TryFrom<&[&Path]> for AdBuffet {
    type Error = anyhow::Error;

    fn try_from(paths: &[&Path]) -> Result<Self, Self::Error> {
        let queue = paths
            .iter()
            .map(|&path| AdEntry::try_from(path).map(Arc::new))
            .collect::<anyhow::Result<_>>()?;

        Ok(Self {
            queue,
            pos: Arc::default(),
        })
    }
}

impl AdBuffet {
    pub fn next(&self) -> Option<AdEntryRef> {
        if self.queue.is_empty() {
            return None;
        }

        self.pos
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                Some((x + 1) % self.queue.len())
            })
            .map_or(None, |pos| {
                self.queue.get(pos).cloned().map(AdEntryRef::from)
            })
    }

    pub fn size(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_name(entry: &AdEntryRef) -> &str {
        entry.name()
    }

    #[test]
    fn test_empty() {
        let sut = AdBuffet::empty();
        assert!(sut.next().is_none());
    }

    #[test]
    fn test_single() {
        let sut = AdBuffet::from(["single"]);
        assert_eq!(sut.next().as_ref().map_or("", AdEntryRef::name), "single");
        assert_eq!(sut.next().as_ref().map_or("", AdEntryRef::name), "single");
    }

    #[test]
    fn test_few() {
        let sut = AdBuffet::from(["first", "second", "third"]);
        assert_eq!(sut.next().as_ref().map_or("", AdEntryRef::name), "first");
        assert_eq!(sut.next().as_ref().map_or("", AdEntryRef::name), "second");
        assert_eq!(sut.next().as_ref().map_or("", AdEntryRef::name), "third");
        assert_eq!(sut.next().as_ref().map_or("", AdEntryRef::name), "first");
        assert_eq!(sut.next().as_ref().map_or("", AdEntryRef::name), "second");
        assert_eq!(sut.next().as_ref().map_or("", AdEntryRef::name), "third");
    }

    impl AdBuffet {
        fn empty() -> Self {
            Self {
                queue: Vec::default(),
                pos: Arc::default(),
            }
        }
    }

    impl<const N: usize> From<[&str; N]> for AdBuffet {
        fn from(names: [&str; N]) -> Self {
            Self {
                queue: names
                    .into_iter()
                    .map(AdEntry::from_name)
                    .map(Arc::new)
                    .collect(),
                pos: Arc::default(),
            }
        }
    }
}
