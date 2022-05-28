use async_trait::async_trait;

use super::SegmentProcessor;
use crate::app::Segment;
use crate::utils;

pub struct DownloadProcessor;

#[async_trait]
impl SegmentProcessor for DownloadProcessor {
    async fn process(mut segment: Segment) -> anyhow::Result<Segment> {
        let (content_type, content) = utils::download(&segment.url).await?;
        if let Some(v) = content_type {
            segment.tags.insert("FileType".to_string(), v);
        }
        Ok(Segment { content, ..segment })
    }
}
