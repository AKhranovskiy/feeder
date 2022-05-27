use anyhow::Context;
use anyhow::Result;
use model::Segment;
use reqwest::Url;
use tokio_stream::StreamExt;

use crate::app::processors::DownloadProcessor;
use crate::app::processors::SegmentProcessor;
use crate::app::processors::TagExtractor;
use crate::app::upload::upload;

mod args;
mod fetchers;
mod processors;
mod upload;

pub use args::Args;
use fetchers::HttpLiveStreamingFetcher;

pub struct App;

impl App {
    pub async fn run(args: Args) -> Result<()> {
        let stream = args.m3u8.parse::<Url>()?;

        let segments = HttpLiveStreamingFetcher::new(stream).fetch();
        tokio::pin!(segments);

        while let Some(segment) = segments.next().await {
            let segment: Segment = segment?;

            let segment = DownloadProcessor::process(segment)
                .await
                .context("Downloading content")?;

            let segment = TagExtractor::process(segment)
                .await
                .context("Extracting tags")?;

            upload(segment).await.context("Uploading a segment")?;
        }
        Ok(())
    }
}
