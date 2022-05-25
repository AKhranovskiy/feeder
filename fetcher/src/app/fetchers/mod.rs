mod httplivestream;

use std::time::Duration;

pub use httplivestream::HttpLiveStreamingFetcher;
use reqwest::Url;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SegmentInfo {
    pub url: Url,
    duration: Duration,
    title: Option<String>,
}
