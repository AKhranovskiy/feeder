use std::io::Cursor;

use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use lofty::{ItemKey, ItemValue, Probe};
use model::Tags;

use super::SegmentProcessor;
use crate::app::Segment;

pub struct TagExtractor;

#[async_trait]
impl SegmentProcessor for TagExtractor {
    async fn process(segment: Segment) -> anyhow::Result<Segment> {
        let mut tags = segment.tags.clone();
        tags.append(&mut extract(&segment.content)?);
        Ok(Segment { tags, ..segment })
    }
}

fn extract(bytes: &Bytes) -> anyhow::Result<Tags> {
    let tagged_file = Probe::new(Cursor::new(bytes))
        .guess_file_type()
        .context("guess file type")?
        .read(false)
        .context("read file")?;

    // TODO https://en.wikipedia.org/wiki/ID3
    let mut tags = Tags::new();
    for tag in tagged_file.tags() {
        for item in tag.items() {
            let key: Option<&str> = match item.key() {
                ItemKey::AlbumTitle => Some("AlbumTitle"),
                ItemKey::Comment => Some("Comment"),
                ItemKey::TrackArtist => Some("TrackArtist"),
                ItemKey::TrackTitle => Some("TrackTitle"),
                ItemKey::OriginalFileName => Some("FileName"),
                ItemKey::Unknown(v) => Some(v.as_str()),
                _ => {
                    log::error!(
                        "Unsupported tag: key={:?}, value={:?}",
                        item.key(),
                        item.value()
                    );
                    None
                }
            };
            if let Some(key) = key {
                let value = match item.value() {
                    ItemValue::Text(v) | ItemValue::Locator(v) => v.clone(),
                    ItemValue::Binary(v) => std::str::from_utf8(v)?.to_owned(),
                };
                // log::debug!("Tag: key={:?}, value={:?}", key, value);
                tags.insert(key.to_owned(), value);
            }
        }
    }
    Ok(tags)
}
