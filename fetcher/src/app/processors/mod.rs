mod download;
mod tag_extractor;

use async_trait::async_trait;

use super::Segment;

#[async_trait]
pub trait SegmentProcessor {
    async fn process(mut segment: Segment) -> anyhow::Result<Segment>;
}

pub use download::DownloadProcessor;
pub use tag_extractor::TagExtractor;
