use std::str::FromStr;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use async_stream::try_stream;
use hls_m3u8::MediaPlaylist;
use reqwest::header::CONTENT_TYPE;
use reqwest::{StatusCode, Url};
use tokio_stream::Stream;

pub struct HttpLiveStreamingFetcher {
    source: Url,
    client: reqwest::Client,
}

impl HttpLiveStreamingFetcher {
    pub fn new(source: Url) -> Self {
        Self {
            source,
            client: reqwest::Client::new(),
        }
    }

    async fn fetch_playlist(&self) -> Result<MediaPlaylist> {
        let response = self.client.get(self.source.clone()).send().await?;

        if response.status() != StatusCode::OK {
            bail!(
                "Failed to get data from source: {} {}",
                response.status(),
                response.text().await?
            );
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .ok_or_else(|| anyhow!("Missing content-type header"))?;

        let content_type = content_type.to_str()?;

        if content_type != "application/vnd.apple.mpegurl; charset=UTF-8" {
            bail!("Invalid content-type header: {}", content_type);
        }

        MediaPlaylist::from_str(response.text().await?.as_ref())
            .map_err(|e| anyhow!("Failed to parse playlist: {:#}", e))
    }

    pub fn fetch(self) -> impl Stream<Item = Result<u64>> {
        log::trace!(target: "HttpLiveStreamingFetcher", "Fetching source={}", &self.source);

        // Uses the `try_stream` macro from the `async-stream` crate. Generators
        // are not stable in Rust. The crate uses a macro to simulate generators
        // on top of async/await. There are limitations, so read the
        // documentation there.
        try_stream! {
            loop {
                let playlist = self.fetch_playlist().await?;
                for (_, segment) in &playlist.segments {
                    yield segment.duration.duration().as_secs()
                }

                tokio::time::sleep(playlist.duration() / 2).await;
            }
        }
    }
}
