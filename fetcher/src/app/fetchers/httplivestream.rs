use std::cell::Cell;
use std::str::FromStr;

use anyhow::{anyhow, bail, Result};
use async_stream::try_stream;
use hls_m3u8::tags::VariantStream::{ExtXIFrame, ExtXStreamInf};
use hls_m3u8::{MasterPlaylist, MediaPlaylist, MediaSegment};
use model::Segment;
use reqwest::Url;
use tokio_stream::Stream;

use crate::utils;

use super::segment_info::SegmentInfo;

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

        let content = std::str::from_utf8(&content)?;
        let uri = match MasterPlaylist::try_from(content) {
            // TODO handle all streams, choose the lowest quality.
            Ok(master_playlist) => master_playlist.variant_streams.first().map(|p| match p {
                ExtXIFrame {
                    uri,
                    stream_data: _,
                }
                | ExtXStreamInf {
                    uri,
                    frame_rate: _,
                    audio: _,
                    subtitles: _,
                    closed_captions: _,
                    stream_data: _,
                } => Url::parse(uri),
            }),
            Err(_) => None,
        };
        if let Some(Ok(uri)) = uri {
            let (content_type, content) = utils::download(&uri).await?;

            if content_type != Some("application/vnd.apple.mpegurl; charset=UTF-8".to_string()) {
                bail!("Invalid content-type header: {:?}", content_type);
            }
            let content = std::str::from_utf8(&content)?;
            MediaPlaylist::from_str(content)
                .map_err(|e| anyhow!("Failed to parse playlist: {:#}", e))
        } else {
            MediaPlaylist::from_str(content)
                .map_err(|e| anyhow!("Failed to parse playlist: {:#}", e))
        }
    }

    pub fn fetch(self) -> impl Stream<Item = Result<Segment>> {
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
                    yield info.into()
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
            title: value
                .duration
                .title()
                .as_ref()
                .map(std::string::ToString::to_string),
        })
    }
}
