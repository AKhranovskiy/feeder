use anyhow::Result;
use bytes::Bytes;
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
        let mut segments = segments.map(|info| {
            info.map(Segment::from)
                .map(|s| async { SegmentDownloader::process(s).await })
        });

        while let Some(segment) = segments.next().await {
            let segment: Segment = segment?.await?;
            log::debug!(
                "segment {:?} {:?}",
                segment.content_type,
                segment.content.map(|c| c.len())
            );
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
