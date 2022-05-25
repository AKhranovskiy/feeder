use std::io::Cursor;

use anyhow::Result;
use bytes::Bytes;
use lofty::Probe;
use reqwest::Url;
use tokio_stream::StreamExt;

mod args;
pub use args::Args;

mod fetchers;
use crate::app::fetchers::HttpLiveStreamingFetcher;
use crate::utils;

use self::fetchers::SegmentInfo;

pub struct App;

impl App {
    pub async fn run(args: Args) -> Result<()> {
        let stream = args.m3u8.parse::<Url>()?;

        let segments = HttpLiveStreamingFetcher::new(stream).fetch();
        tokio::pin!(segments);

        while let Some(info) = segments.next().await {
            let segment: Segment = info?.into();
            let segment = SegmentDownloader::process(segment).await?;
            let segment = TagExtractor::process(segment).await?;

            log::debug!("segment {:?}", segment.tags,);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct Segment {
    info: SegmentInfo,
    content: Option<Bytes>,
    content_type: Option<String>,
    tags: Option<Vec<SegmentTags>>,
}

impl From<SegmentInfo> for Segment {
    fn from(info: SegmentInfo) -> Self {
        Self {
            info,
            content: None,
            content_type: None,
            tags: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
struct SegmentTags {
    title: String,
    artist: String,
    other: String,
}

struct SegmentDownloader;

impl SegmentDownloader {
    async fn process(segment: Segment) -> Result<Segment> {
        let (content_type, content) = utils::download(&segment.info.url).await?;
        Ok(Segment {
            content: Some(content),
            content_type,
            ..segment
        })
    }
}

struct TagExtractor;

impl TagExtractor {
    fn extract(bytes: &Bytes) -> Result<Option<SegmentTags>> {
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
    async fn process(segment: Segment) -> Result<Segment> {
        let tags = segment
            .content
            .as_ref()
            .and_then(|bytes| Self::extract(bytes).ok().flatten());

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
