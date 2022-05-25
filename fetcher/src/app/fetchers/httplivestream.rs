use std::cell::Cell;
use std::str::FromStr;

use anyhow::{anyhow, bail, Result};
use async_stream::try_stream;
use hls_m3u8::{MediaPlaylist, MediaSegment};
use reqwest::Url;
use tokio_stream::Stream;

use crate::app::segment::SegmentInfo;
use crate::utils;

pub struct HttpLiveStreamingFetcher {
    source: Url,
    last_seen_segment_number: Cell<usize>,
}

impl HttpLiveStreamingFetcher {
    pub fn new(source: Url) -> Self {
        Self {
            source,
            last_seen_segment_number: Cell::new(0),
        }
    }

    async fn fetch_playlist(&self) -> Result<MediaPlaylist> {
        let (content_type, content) = utils::download(&self.source).await?;

        if content_type != Some("application/vnd.apple.mpegurl; charset=UTF-8".to_string()) {
            bail!("Invalid content-type header: {:?}", content_type);
        }

        MediaPlaylist::from_str(std::str::from_utf8(&content)?)
            .map_err(|e| anyhow!("Failed to parse playlist: {:#}", e))
    }

    pub fn fetch(self) -> impl Stream<Item = Result<SegmentInfo>> {
        log::trace!(target: "HttpLiveStreamingFetcher", "Fetching source={}", &self.source);

        // Uses the `try_stream` macro from the `async-stream` crate. Generators
        // are not stable in Rust. The crate uses a macro to simulate generators
        // on top of async/await. There are limitations, so read the
        // documentation there.
        try_stream! {
            loop {
                let playlist = self.fetch_playlist().await?;
                for (_, segment) in playlist.segments.iter().filter(|(_, s)| self.filter_segment(s)) {
                    let info: SegmentInfo= segment.try_into()?;
                    yield info
                }

                tokio::time::sleep(playlist.duration() / 2).await;
            }
        }
    }

    fn filter_segment(&self, segment: &MediaSegment) -> bool {
        let number = segment.number();
        if number <= self.last_seen_segment_number.get() {
            false
        } else {
            self.last_seen_segment_number.set(number);
            true
        }
    }
}

impl TryFrom<&MediaSegment<'_>> for SegmentInfo {
    type Error = anyhow::Error;

    fn try_from(value: &MediaSegment) -> Result<Self, Self::Error> {
        Ok(SegmentInfo {
            url: value.uri().parse()?,
            duration: value.duration.duration(),
            title: value.duration.title().as_ref().map(|t| t.to_string()),
        })
    }
}
