use std::io::Cursor;

use async_trait::async_trait;
use bytes::Bytes;
use lofty::Probe;

use super::SegmentProcessor;
use crate::app::segment::SegmentTags;
use crate::app::Segment;

pub struct TagExtractor;

#[async_trait]
impl SegmentProcessor for TagExtractor {
    async fn process(segment: Segment) -> anyhow::Result<Segment> {
        let tags = segment
            .content
            .as_ref()
            .and_then(|bytes| extract(bytes).ok().flatten());

        if let Some(tags) = tags {
            if let Some(mut v) = segment.tags {
                v.push(tags);
                Ok(Segment {
                    tags: Some(v),
                    ..segment
                })
            } else {
                Ok(Segment {
                    tags: Some(vec![tags]),
                    ..segment
                })
            }
        } else {
            Ok(segment)
        }
    }
}

fn extract(bytes: &Bytes) -> anyhow::Result<Option<SegmentTags>> {
    let tagged_file = Probe::new(Cursor::new(bytes))
        .guess_file_type()?
        .read(false)?;

    // TODO https://en.wikipedia.org/wiki/ID3#ID3v2
    // TXXX WXXX
    for tag in tagged_file.tags() {
        for item in tag.items() {
            log::info!("{:?} {:?}", item.key(), item.value());
        }
    }
    Ok(None)
}
