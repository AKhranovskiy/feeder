use std::str::from_utf8;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_stream::try_stream;
use futures::future::join_all;
use futures::stream::Stream;
use hls_m3u8::tags::VariantStream;
use hls_m3u8::{MasterPlaylist, MediaPlaylist, MediaSegment};

mod downloader;
mod playlist;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Segment {
    pub content: Vec<u8>,
    pub content_type: String,
    pub duration: Duration,
    pub source: String,
    pub comment: String,
}

pub async fn fetch(source: String) -> impl Stream<Item = anyhow::Result<Segment>> + Unpin {
    fetch_impl(source, downloader::UReqDl).await
}

const HLS_CONTENT_TYPE: &str = "application/vnd.apple.mpegurl";

async fn fetch_impl(
    source: String,
    dl: impl downloader::Downloader,
) -> impl Stream<Item = anyhow::Result<Segment>> + Unpin {
    // Download source.
    // If it is master playlist
    // Repeatedly download media playlist
    // Yeild segments.
    // If it is media playlist, yield segments.
    let last_sequence_number = Arc::new(AtomicUsize::new(0));

    Box::pin(try_stream! {

        // Go through master playlist.
        let playlist = download_hls(&dl, source.clone()).await?;
        let source = get_media_playlist_url(&playlist).unwrap_or(source);

        loop {
            // TODO - avoid double download if original source points to media playlist.
            let playlist = download_hls(&dl, source.clone()).await?;

            let segments = join_all(
                extract_segments(&playlist)?
                .iter()
                .filter(|s| s.sequence_number > last_sequence_number.load(Ordering::Relaxed))
                .cloned()
                .map(|_|async {
                        anyhow::Ok(Segment{
                            content: vec![],
                            content_type: String::default(),
                            duration: Duration::default(),
                            source: String::default(),
                            comment: String::default()})
                        }
                    // TODO fix
                    // async {dl.download(s.source.clone()).await
                    //     .map(|(content_type, content)| {
                    //         last_sequence_number.fetch_max(s.sequence_number, Ordering::SeqCst);
                    //         Segment { content, content_type, duration: s.duration, source: s.source, comment: s.title}
                    //     }) }
                )
            ).await;

            for segment in segments {
                yield segment?;
            }
        }
    })
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SegmentInfo {
    sequence_number: usize,
    duration: Duration,
    source: String,
    title: String,
}
impl From<&MediaSegment<'_>> for SegmentInfo {
    fn from(segment: &MediaSegment) -> Self {
        Self {
            sequence_number: segment.number(),
            duration: segment.duration.duration(),
            source: segment.uri().to_string(),
            title: segment
                .duration
                .title()
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        }
    }
}

fn extract_segments(content: &[u8]) -> anyhow::Result<Vec<SegmentInfo>> {
    from_utf8(content)
        .map_err(|e| anyhow::anyhow!(e))
        .and_then(|s| MediaPlaylist::try_from(s).map_err(Into::into))
        .map(|playlist| {
            playlist
                .segments
                .iter()
                .map(|(_, segment)| segment.into())
                .collect()
        })
}

async fn download_hls(dl: &impl downloader::Downloader, source: String) -> anyhow::Result<Vec<u8>> {
    let (content_type, content) = dl.download(source).await?;

    if content_type == HLS_CONTENT_TYPE {
        Ok(content)
    } else {
        Err(anyhow::anyhow!("Invalid content type: {content_type}"))
    }
}

fn get_media_playlist_url(content: &[u8]) -> Option<String> {
    from_utf8(content)
        .map_err(|e| anyhow::anyhow!(e))
        .and_then(|s| MasterPlaylist::try_from(s).map_err(Into::into))
        .ok()
        .as_ref()
        .and_then(get_best_stream_url)
}

fn get_best_stream_url(playlist: &MasterPlaylist) -> Option<String> {
    playlist
        .variant_streams
        .iter()
        .filter_map(|vs| match vs {
            VariantStream::ExtXIFrame { .. } => None,
            VariantStream::ExtXStreamInf {
                uri, stream_data, ..
            } => Some((uri.to_string(), stream_data.bandwidth())),
        })
        .max_by_key(|(_, bandwidth)| *bandwidth)
        .map(|(uri, _)| uri)
}

#[cfg(test)]
mod tests {
    use futures::{StreamExt, TryStreamExt};

    use crate::downloader::TestDl;
    use crate::playlist::{make_master_playlist, make_media_playlist, SegmentOrder};
    use crate::{fetch_impl, HLS_CONTENT_TYPE};

    #[tokio::test]
    async fn test_fetch_invalid_source() {
        let dl = TestDl::new(vec![]);

        let segments = fetch_impl("some url".to_string(), dl)
            .await
            .try_collect::<Vec<_>>()
            .await;

        assert!(segments.is_err());
        println!("ERROR: {:?}", segments.unwrap_err());
    }

    #[tokio::test]
    async fn test_fetch_master_wrong_content_type() {
        let dl = TestDl::new(vec![("plain/txt".to_string(), make_master_playlist())]);

        let segments = fetch_impl("some url".to_string(), dl)
            .await
            .try_collect::<Vec<_>>()
            .await;

        assert!(segments.is_err());
        println!("ERROR: {:?}", segments.unwrap_err());
    }

    #[tokio::test]
    async fn test_fetch_master_without_children() {
        let dl = TestDl::new(vec![(HLS_CONTENT_TYPE.to_string(), make_master_playlist())]);

        let segments = fetch_impl("some url".to_string(), dl)
            .await
            .try_collect::<Vec<_>>()
            .await;

        assert!(segments.is_err());
        println!("ERROR: {:?}", segments.unwrap_err());
    }

    #[tokio::test]
    async fn test_fetch_children_wrong_type() {
        let dl = TestDl::new(vec![
            (HLS_CONTENT_TYPE.to_string(), make_master_playlist()),
            (
                "plain/html".to_string(),
                make_media_playlist(1, 1, SegmentOrder::Direct),
            ),
        ]);

        let segments = fetch_impl("some url".to_string(), dl)
            .await
            .try_collect::<Vec<_>>()
            .await;

        assert!(segments.is_err());
        println!("ERROR: {:?}", segments.unwrap_err());
    }

    #[tokio::test]
    async fn test_fetch_children_no_segments() {
        let dl = TestDl::new(vec![
            (HLS_CONTENT_TYPE.to_string(), make_master_playlist()),
            (
                HLS_CONTENT_TYPE.to_string(),
                make_media_playlist(1, 0, SegmentOrder::Direct),
            ),
        ]);

        let segments = fetch_impl("some url".to_string(), dl)
            .await
            .try_collect::<Vec<_>>()
            .await;

        assert!(segments.is_err());
        println!("ERROR: {:?}", segments.unwrap_err());
    }

    #[tokio::test]
    async fn test_fetch_segments() {
        let dl = TestDl::new(vec![
            (HLS_CONTENT_TYPE.to_string(), make_master_playlist()),
            (
                HLS_CONTENT_TYPE.to_string(),
                make_media_playlist(1, 3, SegmentOrder::Direct),
            ),
            ("media/aac".to_string(), "1".as_bytes().to_vec()),
            ("media/aac".to_string(), "2".as_bytes().to_vec()),
            ("media/aac".to_string(), "3".as_bytes().to_vec()),
            (
                HLS_CONTENT_TYPE.to_string(),
                make_media_playlist(2, 3, SegmentOrder::Direct),
            ),
            ("media/aac".to_string(), "2".as_bytes().to_vec()),
            ("media/aac".to_string(), "3".as_bytes().to_vec()),
            ("media/aac".to_string(), "4".as_bytes().to_vec()),
        ]);

        let segments = fetch_impl("some url".to_string(), dl)
            .await
            .collect::<Vec<_>>()
            .await;

        println!("{segments:#?}");
        assert_eq!(segments.len(), 5);
        assert!(segments[0..4].iter().all(std::result::Result::is_ok));
        assert!(segments[4].is_err());
    }

    #[tokio::test]
    async fn test_fetch_segments_reversed_order() {
        let dl = TestDl::new(vec![
            (HLS_CONTENT_TYPE.to_string(), make_master_playlist()),
            (
                HLS_CONTENT_TYPE.to_string(),
                make_media_playlist(1, 3, SegmentOrder::Reversed),
            ),
            ("media/aac".to_string(), "3".as_bytes().to_vec()),
            ("media/aac".to_string(), "2".as_bytes().to_vec()),
            ("media/aac".to_string(), "1".as_bytes().to_vec()),
            (
                HLS_CONTENT_TYPE.to_string(),
                make_media_playlist(2, 3, SegmentOrder::Reversed),
            ),
            ("media/aac".to_string(), "4".as_bytes().to_vec()),
            ("media/aac".to_string(), "3".as_bytes().to_vec()),
            ("media/aac".to_string(), "2".as_bytes().to_vec()),
        ]);

        let segments = fetch_impl("some url".to_string(), dl)
            .await
            .collect::<Vec<_>>()
            .await;

        println!("REV {segments:?}");
        assert_eq!(segments.len(), 5);
        assert!(segments[0..4].iter().all(Result::is_ok));
        assert!(segments[4].is_err());
    }
}
