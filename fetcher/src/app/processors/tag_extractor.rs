use std::io::Cursor;

use async_trait::async_trait;
use bytes::Bytes;
use lofty::Probe;
use model::Tags;

use super::SegmentProcessor;
use crate::app::Segment;

pub struct TagExtractor;

#[async_trait]
impl SegmentProcessor for TagExtractor {
    async fn process(mut segment: Segment) -> anyhow::Result<Segment> {
        let mut tags = extract(&segment.content)?;
        segment.tags.append(&mut tags);
        Ok(segment)
    }
}

fn extract(bytes: &Bytes) -> anyhow::Result<Tags> {
    let tagged_file = Probe::new(Cursor::new(bytes))
        .guess_file_type()?
        .read(false)?;

    // TODO https://en.wikipedia.org/wiki/ID3
    let mut tags = Tags::new();
    for tag in tagged_file.tags() {
        for item in tag.items() {
            // log::debug!("{:?} {:?}", item.key(), item.value());
            let key: Option<&str> = match item.key() {
                lofty::ItemKey::AlbumTitle => Some("AlbumTitle"),
                lofty::ItemKey::TrackArtist => Some("TrackArtist"),
                lofty::ItemKey::TrackTitle => Some("TrackTitle"),
                lofty::ItemKey::Unknown(v) => Some(v.as_str()),
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
                    lofty::ItemValue::Text(v) => v.clone(),
                    lofty::ItemValue::Locator(v) => v.clone(),
                    lofty::ItemValue::Binary(v) => std::str::from_utf8(v)?.to_owned(),
                };
                tags.insert(key.to_owned(), value);
            }
        }
    }
    Ok(tags)
}
