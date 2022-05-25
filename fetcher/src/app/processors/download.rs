use async_trait::async_trait;

use super::SegmentProcessor;
use crate::app::Segment;
use crate::utils;

pub struct DownloadProcessor;

#[async_trait]
impl SegmentProcessor for DownloadProcessor {
    async fn process(segment: Segment) -> anyhow::Result<Segment> {
        let (content_type, content) = utils::download(&segment.info.url).await?;
        Ok(Segment {
            content: Some(content),
            content_type,
            ..segment
        })
    }
}
