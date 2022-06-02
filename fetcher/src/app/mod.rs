use anyhow::Context;
use anyhow::Result;
use futures::TryFutureExt;
use model::Segment;
use model::SegmentUploadResponse;
use reqwest::Url;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
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
        let retry_strategy = ExponentialBackoff::from_millis(1000)
            .map(jitter) // add jitter to delays
            .take(3); // limit to 3 retries

        Retry::spawn(retry_strategy, || Self::handle_hls(&args)).await?;
        Ok(())
    }

    async fn handle_hls(args: &Args) -> Result<()> {
        let endpoint = args.endpoint.parse::<Url>()?;
        let stream = args.m3u8.parse::<Url>()?;

        let segments = HttpLiveStreamingFetcher::new(stream).fetch();
        tokio::pin!(segments);

        while let Some(segment) = segments.next().await {
            let segment: Segment = segment?;

            match Self::process_segment(segment)
                .and_then(|segment| upload(&endpoint, segment))
                .await
            {
                Ok(response) => match response {
                    SegmentUploadResponse::Matched(matches) => {
                        log::info!("Matched:");
                        for m in &matches {
                            log::info!(
                                "\t{}% {} / {:?} / {} / {}",
                                u16::from(m.score) * 100 / 255,
                                m.id,
                                m.kind,
                                m.artist,
                                m.title
                            );
                        }
                    }
                    SegmentUploadResponse::Inserted(r) => {
                        log::info!("New: {} / {:?} / {} / {}", r.id, r.kind, r.artist, r.title);
                    }
                },

                Err(e) => {
                    log::error!("{e:#}");
                }
            };
        }
        Ok(())
    }

    async fn process_segment(segment: Segment) -> Result<Segment> {
        let segment = DownloadProcessor::process(segment)
            .await
            .context("Downloading content")?;

        let segment = TagExtractor::process(segment)
            .await
            .context("Extracting tags")?;

        Ok(segment)
    }
}
