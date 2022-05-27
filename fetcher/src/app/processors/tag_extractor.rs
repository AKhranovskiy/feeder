use std::io::Cursor;

use async_trait::async_trait;
use bytes::Bytes;
use lofty::Probe;

use super::SegmentProcessor;
use crate::app::segment::Tags;
use crate::app::Segment;

pub struct TagExtractor;

#[async_trait]
impl SegmentProcessor for TagExtractor {
    async fn process(mut segment: Segment) -> anyhow::Result<Segment> {
        let mut tags = segment
            .content
            .as_ref()
            .and_then(|bytes| extract(bytes).ok())
            .unwrap_or_default();

        segment.tags.append(&mut tags);
        Ok(segment)
    }
}

fn extract(bytes: &Bytes) -> anyhow::Result<Tags> {
    let tagged_file = Probe::new(Cursor::new(bytes))
        .guess_file_type()?
        .read(false)?;

    // TODO https://en.wikipedia.org/wiki/ID3#ID3v2
    // TXXX WXXX
    let mut tags = Tags::new();
    for tag in tagged_file.tags() {
        for item in tag.items() {
            // log::debug!("{:?} {:?}", item.key(), item.value());
            let key: Option<&str> = match item.key() {
                lofty::ItemKey::AlbumArtist => todo!(),
                lofty::ItemKey::AlbumArtistSortOrder => todo!(),
                lofty::ItemKey::AlbumTitle => Some("AlbumTitle"),
                lofty::ItemKey::AlbumTitleSortOrder => todo!(),
                lofty::ItemKey::Arranger => todo!(),
                lofty::ItemKey::AudioFileURL => todo!(),
                lofty::ItemKey::AudioSourceURL => todo!(),
                lofty::ItemKey::BPM => todo!(),
                lofty::ItemKey::Barcode => todo!(),
                lofty::ItemKey::CatalogNumber => todo!(),
                lofty::ItemKey::Comment => todo!(),
                lofty::ItemKey::CommercialInformationURL => todo!(),
                lofty::ItemKey::Composer => todo!(),
                lofty::ItemKey::ComposerSortOrder => todo!(),
                lofty::ItemKey::Conductor => todo!(),
                lofty::ItemKey::ContentGroup => todo!(),
                lofty::ItemKey::CopyrightMessage => todo!(),
                lofty::ItemKey::CopyrightURL => todo!(),
                lofty::ItemKey::Description => todo!(),
                lofty::ItemKey::DiscNumber => todo!(),
                lofty::ItemKey::DiscTotal => todo!(),
                lofty::ItemKey::EncodedBy => todo!(),
                lofty::ItemKey::EncoderSettings => todo!(),
                lofty::ItemKey::EncoderSoftware => todo!(),
                lofty::ItemKey::EncodingTime => todo!(),
                lofty::ItemKey::Engineer => todo!(),
                lofty::ItemKey::FileOwner => todo!(),
                lofty::ItemKey::FileType => todo!(),
                lofty::ItemKey::FlagCompilation => todo!(),
                lofty::ItemKey::FlagPodcast => todo!(),
                lofty::ItemKey::Genre => todo!(),
                lofty::ItemKey::ISRC => todo!(),
                lofty::ItemKey::InitialKey => todo!(),
                lofty::ItemKey::InternetRadioStationName => todo!(),
                lofty::ItemKey::InternetRadioStationOwner => todo!(),
                lofty::ItemKey::InvolvedPeople => todo!(),
                lofty::ItemKey::Label => todo!(),
                lofty::ItemKey::Language => todo!(),
                lofty::ItemKey::Length => todo!(),
                lofty::ItemKey::License => todo!(),
                lofty::ItemKey::Lyricist => todo!(),
                lofty::ItemKey::Lyrics => todo!(),
                lofty::ItemKey::MixDj => todo!(),
                lofty::ItemKey::MixEngineer => todo!(),
                lofty::ItemKey::Mood => todo!(),
                lofty::ItemKey::Movement => todo!(),
                lofty::ItemKey::MovementIndex => todo!(),
                lofty::ItemKey::MusicianCredits => todo!(),
                lofty::ItemKey::OriginalAlbumTitle => todo!(),
                lofty::ItemKey::OriginalArtist => todo!(),
                lofty::ItemKey::OriginalFileName => todo!(),
                lofty::ItemKey::OriginalLyricist => todo!(),
                lofty::ItemKey::OriginalMediaType => todo!(),
                lofty::ItemKey::OriginalReleaseDate => todo!(),
                lofty::ItemKey::ParentalAdvisory => todo!(),
                lofty::ItemKey::PaymentURL => todo!(),
                lofty::ItemKey::Performer => todo!(),
                lofty::ItemKey::PodcastDescription => todo!(),
                lofty::ItemKey::PodcastGlobalUniqueID => todo!(),
                lofty::ItemKey::PodcastKeywords => todo!(),
                lofty::ItemKey::PodcastReleaseDate => todo!(),
                lofty::ItemKey::PodcastSeriesCategory => todo!(),
                lofty::ItemKey::PodcastURL => todo!(),
                lofty::ItemKey::Popularimeter => todo!(),
                lofty::ItemKey::Producer => todo!(),
                lofty::ItemKey::Publisher => todo!(),
                lofty::ItemKey::PublisherURL => todo!(),
                lofty::ItemKey::RadioStationURL => todo!(),
                lofty::ItemKey::RecordingDate => todo!(),
                lofty::ItemKey::Remixer => todo!(),
                lofty::ItemKey::Script => todo!(),
                lofty::ItemKey::SetSubtitle => todo!(),
                lofty::ItemKey::ShowName => todo!(),
                lofty::ItemKey::ShowNameSortOrder => todo!(),
                lofty::ItemKey::TaggingTime => todo!(),
                lofty::ItemKey::TrackArtist => Some("TrackArtist"),
                lofty::ItemKey::TrackArtistSortOrder => todo!(),
                lofty::ItemKey::TrackArtistURL => todo!(),
                lofty::ItemKey::TrackNumber => todo!(),
                lofty::ItemKey::TrackSubtitle => todo!(),
                lofty::ItemKey::TrackTitle => Some("TrackTitle"),
                lofty::ItemKey::TrackTitleSortOrder => todo!(),
                lofty::ItemKey::TrackTotal => todo!(),
                lofty::ItemKey::Unknown(v) => Some(v.as_str()),
                lofty::ItemKey::Writer => todo!(),
                lofty::ItemKey::Year => todo!(),
                _ => {
                    log::error!("Unknown tag key: {:?} {:?}", item.key(), item.value());
                    None
                }
            };
            if let Some(key) = key {
                let value = match item.value() {
                    lofty::ItemValue::Text(v) => v.clone(),
                    lofty::ItemValue::Locator(v) => v.clone(),
                    lofty::ItemValue::Binary(v) => std::str::from_utf8(v)?.to_owned(),
                };
                tags.insert(key.to_owned(), value);
            }
        }
    }
    Ok(tags)
}
