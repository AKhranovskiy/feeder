#![allow(dead_code)]

use std::cell::Cell;

#[derive(Debug, Clone)]
pub struct AdBuffet {
    queue: Vec<AdEntry>,
    pos: Cell<usize>,
}

#[derive(Debug, Clone)]
pub struct AdEntry {
    pub name: String,
}

impl From<&str> for AdEntry {
    fn from(name: &str) -> Self {
        Self { name: name.into() }
    }
}

impl AdBuffet {
    pub fn new() -> Self {
        Self {
            queue: vec![],
            pos: Cell::new(0),
        }
    }

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

impl FromIterator<AdEntry> for AdBuffet {
    fn from_iter<T: IntoIterator<Item = AdEntry>>(iter: T) -> Self {
        Self {
            queue: Vec::from_iter(iter),
            pos: Cell::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let sut = AdBuffet::new();
        assert!(sut.next().is_none());
    }

    #[test]
    fn test_single() {
        let sut = AdBuffet::from_iter([AdEntry::from("single")]);
        assert_eq!(sut.next().map_or("", |a| &a.name), "single");
        assert_eq!(sut.next().map_or("", |a| &a.name), "single");
    }

    #[test]
    fn test_few() {
        let sut = AdBuffet::from_iter([
            AdEntry::from("first"),
            AdEntry::from("second"),
            AdEntry::from("third"),
        ]);
        assert_eq!(sut.next().map_or("", |a| &a.name), "first");
        assert_eq!(sut.next().map_or("", |a| &a.name), "second");
        assert_eq!(sut.next().map_or("", |a| &a.name), "third");
        assert_eq!(sut.next().map_or("", |a| &a.name), "first");
        assert_eq!(sut.next().map_or("", |a| &a.name), "second");
        assert_eq!(sut.next().map_or("", |a| &a.name), "third");
    }
}
