use async_trait::async_trait;

use super::SegmentProcessor;
use crate::app::Segment;
use crate::utils;

pub struct DownloadProcessor;

#[async_trait]
impl SegmentProcessor for DownloadProcessor {
    async fn process(segment: Segment) -> anyhow::Result<Segment> {
        let (content_type, content) = utils::download(&segment.url).await?;
        Ok(Segment {
            content,
            content_type: content_type.unwrap_or_default(),
            ..segment
        })
    }
}
