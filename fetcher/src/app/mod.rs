use anyhow::{Context, Result};
use reqwest::Url;
use tokio_stream::StreamExt;

mod args;
pub use self::args::Args;

mod fetchers;
use self::fetchers::HttpLiveStreamingFetcher;

pub struct App;

impl App {
    pub async fn run(args: Args) -> Result<()> {
        let stream = args.m3u8.parse::<Url>()?;

        let segments = HttpLiveStreamingFetcher::new(stream).fetch();
        tokio::pin!(segments);

        while let Some(segment) = segments.next().await {
            println!("segment = {segment:?}");
        }
        Ok(())
    }
}
