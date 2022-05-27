use anyhow::Result;
use model::Segment;
use reqwest::Url;
use tokio_stream::StreamExt;

use crate::app::processors::DownloadProcessor;
use crate::app::processors::SegmentProcessor;
use crate::app::processors::TagExtractor;

mod args;
mod fetchers;
mod processors;

pub use args::Args;
use fetchers::HttpLiveStreamingFetcher;

pub struct App;

impl App {
    pub async fn run(args: Args) -> Result<()> {
        let stream = args.m3u8.parse::<Url>()?;

        let segments = HttpLiveStreamingFetcher::new(stream).fetch();
        tokio::pin!(segments);

        while let Some(info) = segments.next().await {
            let segment: Segment = info?.into();
            let segment = DownloadProcessor::process(segment).await?;
            let segment = TagExtractor::process(segment).await?;

            log::debug!("segment {:?}", segment.tags,);
        }
        Ok(())
    }
}
