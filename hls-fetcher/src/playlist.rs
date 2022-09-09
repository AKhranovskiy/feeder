#[cfg(test)]
pub fn make_master_playlist() -> Vec<u8> {
    use hls_m3u8::tags::VariantStream;
    use hls_m3u8::types::StreamData;
    use hls_m3u8::MasterPlaylist;

    MasterPlaylist::builder()
        .variant_streams(vec![VariantStream::ExtXStreamInf {
            uri: "https://example.com/playlist.m3u8".into(),
            frame_rate: None,
            audio: None,
            subtitles: None,
            closed_captions: None,
            stream_data: StreamData::builder()
                .bandwidth(22050)
                .codecs(["mp4a.40.2"])
                .build()
                .expect("Valid stream data"),
        }])
        .has_independent_segments(true)
        .build()
        .expect("Valid master playlist")
        .to_string()
        .as_bytes()
        .to_vec()
}

#[cfg(test)]
pub enum SegmentOrder {
    Direct,
    Reversed,
}

#[cfg(test)]
pub fn make_media_playlist(
    sequence_number: usize,
    number_of_segments: usize,
    order: SegmentOrder,
) -> Vec<u8> {
    use std::time::Duration;

    use hls_m3u8::tags::ExtInf;
    use hls_m3u8::{MediaPlaylist, MediaSegment};

    let seq_numbers: Vec<usize> = match order {
        SegmentOrder::Direct => (sequence_number..(sequence_number + number_of_segments)).collect(),
        SegmentOrder::Reversed => (sequence_number..(sequence_number + number_of_segments))
            .rev()
            .collect(),
    };

    MediaPlaylist::builder()
        .target_duration(Duration::from_secs(7))
        .media_sequence(sequence_number)
        .segments(
            seq_numbers
                .into_iter()
                .map(|n| {
                    (
                        n,
                        ExtInf::with_title(
                            Duration::from_secs(7),
                            format!("Title for segment {}", n),
                        ),
                    )
                })
                .map(|(n, inf)| {
                    MediaSegment::builder()
                        .duration(inf)
                        .uri(format!("https://example.com/media/{n}.aac"))
                        .build()
                        .expect("Valid media segment")
                })
                .collect(),
        )
        .build()
        .expect("Valid media playlist")
        .to_string()
        .as_bytes()
        .to_vec()
}

#[cfg(test)]
mod tests {
    use std::str::from_utf8;

    use super::SegmentOrder;
    use super::{make_master_playlist, make_media_playlist};

    #[test]
    fn test_master_playlist() {
        println!("{}", from_utf8(&make_master_playlist()).unwrap());
    }

    #[test]
    fn test_media_playlist() {
        println!(
            "{}",
            from_utf8(&make_media_playlist(5, 3, SegmentOrder::Direct)).unwrap()
        );
    }
    #[test]
    fn test_media_playlist_reversed() {
        println!(
            "{}",
            from_utf8(&make_media_playlist(5, 3, SegmentOrder::Reversed)).unwrap()
        );
    }
}
