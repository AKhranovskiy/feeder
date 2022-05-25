mod httplivestream;

use std::time::Duration;

pub use httplivestream::HttpLiveStreamingFetcher;
use reqwest::Url;

#[derive(Debug, Clone)]
pub struct SegmentInfo {
    url: Url,
    duration: Duration,
    title: Option<String>,
}
