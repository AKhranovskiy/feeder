use anyhow::Result;
use reqwest::Url;
use tokio_stream::StreamExt;

mod args;
pub use args::Args;

mod fetchers;
use crate::app::fetchers::HttpLiveStreamingFetcher;

pub struct App;

impl App {
    pub async fn run(args: Args) -> Result<()> {
        let stream = args.m3u8.parse::<Url>()?;

        let segments = HttpLiveStreamingFetcher::new(stream).fetch();
        tokio::pin!(segments);

        while let Some(segment) = segments.next().await {
            log::debug!("segment = {segment:?}");
        }
        Ok(())
    }
}
