use std::collections::HashMap;
use std::convert::TryFrom;
use std::time::Duration;

use anyhow::anyhow;
use itertools::Itertools;
use lazy_static::lazy_static;

use model::{ContentKind, Tags};
use regex::Regex;

pub struct IHeartRadioGuesser;

impl super::ContentKindGuesser for IHeartRadioGuesser {
    fn guess(&self, tags: &model::Tags) -> Option<ContentKind> {
        if is_promo_project(tags) {
            Some(ContentKind::Advertisement)
        } else {
            let artist = tags.track_artist();
            let title = tags.track_artist();

            let kinds = ["Comment", "TXXX", "URL", "WXXX"]
                .into_iter()
                .inspect(|s| log::debug!("Getting tag {s}"))
                .flat_map(|name| tags.get(name))
                // TODO - keep error.
                .flat_map(|tag| Ihr::try_from(tag).ok())
                .flat_map(|info| info.guess_kind(artist, title))
                .unique()
                .collect_vec();

            match kinds.len() {
                0 => None,
                1 => kinds.first().cloned(),
                _ => {
                    log::error!("IHeartRadioGuesser detected multiple kinds: {kinds:?}");
                    None
                }
            }
        }
    }
}

fn is_promo_project(tags: &Tags) -> bool {
    const PROMO_PREFIXES: &[&str] = &[
        "Iheart Promo Project",
        "Ihm Promo Product",
        "iHR ",
        "IHTU ",
        "ISWI ",
        "ISWI_",
        "OTPC ",
        "STFB ",
        "INSW-",
        "Podcast Promo ",
    ];

    tags.get("TrackTitle").map_or(false, |title| {
        PROMO_PREFIXES.iter().any(|p| title.starts_with(p))
    })
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum SongSpot {
    F,
    M,
    T,
}

impl TryFrom<&str> for SongSpot {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.chars().next() {
            Some('F') => Ok(Self::F),
            Some('M') => Ok(Self::M),
            Some('T') => Ok(Self::T),
            Some(x) => Err(anyhow!("unknown value={x}")),
            None => Err(anyhow!("empty value")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Artist<'s>(&'s str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Title<'s>(&'s str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Id(i64);

impl Id {
    const INVALID: Id = Id(-1);
    const ZERO: Id = Id(0);

    fn is_valid(&self) -> bool {
        self.0 > 0
    }
}

impl TryFrom<&str> for Id {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse().map(Self).map_err(|e| e.into())
    }
}

type AmgArtistId = Id;
type AmgTrackId = Id;
type CartCutId = Id;
type ITunesTrackId = Id;
type MediaBaseId = Id;
type TAId = Id;
type TPId = Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AmgArtworkUrl<'s> {
    Null,
    Url(&'s str),
}

impl<'s> TryFrom<&'s str> for AmgArtworkUrl<'s> {
    type Error = anyhow::Error;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        match value {
            "null" => Ok(Self::Null),
            _ => url::Url::parse(value)
                .map_err(|e| e.into())
                .map(|_| Self::Url(value)),
        }
    }
}

impl<'s> AmgArtworkUrl<'s> {
    fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    fn is_url(&self) -> bool {
        matches!(self, Self::Url(..))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Length(Duration);

impl TryFrom<&str> for Length {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parts = value
            .split(':')
            .take(3)
            .map(str::parse)
            .collect::<Result<Vec<u64>, _>>()?;
        if parts.len() == 3 {
            Ok(Self(Duration::from_secs(
                parts[0] * 3600 + parts[1] * 60 + parts[2],
            )))
        } else {
            Err(anyhow!("expected `hh::mm::ss`, got {value}"))
        }
    }
}

impl Length {
    const ZERO: Length = Length(Duration::ZERO);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdOrUuid<'s> {
    Id(Id),
    Uuid(&'s str),
}

impl<'s> IdOrUuid<'s> {
    #[allow(dead_code)]
    const ZERO: IdOrUuid<'static> = IdOrUuid::Id(Id::ZERO);
    const INVALID: IdOrUuid<'static> = IdOrUuid::Id(Id::INVALID);

    #[allow(dead_code)]
    fn uuid(value: &'s str) -> Self {
        assert!(uuid::Uuid::parse_str(value).is_ok());
        Self::Uuid(value)
    }

    #[allow(dead_code)]
    fn is_id(&self) -> bool {
        matches!(self, Self::Id(..))
    }

    fn is_uuid(&self) -> bool {
        matches!(self, Self::Uuid(..))
    }
}

impl<'s> TryFrom<&'s str> for IdOrUuid<'s> {
    type Error = anyhow::Error;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        if value.len() < 12 {
            Id::try_from(value).map(Self::Id)
        } else {
            uuid::Uuid::parse_str(value)
                .map_err(|e| e.into())
                .map(|_| Self::Uuid(value))
        }
    }
}

type SpotInstanceId<'a> = IdOrUuid<'a>;
type SpEventId<'a> = IdOrUuid<'a>;

#[derive(Debug)]
struct Ihr<'s> {
    amg_artist_id: Option<AmgArtistId>,
    amg_artwork_url: Option<AmgArtworkUrl<'s>>,
    amg_track_id: Option<AmgTrackId>,
    artist: Option<Artist<'s>>,
    cartcut_id: Option<CartCutId>,
    itunes_track_id: Option<ITunesTrackId>,
    length: Option<Length>,
    media_base_id: Option<MediaBaseId>,
    song_spot: Option<SongSpot>,
    #[allow(dead_code)]
    sp_event_id: Option<SpEventId<'s>>,
    spot_instance_id: Option<SpotInstanceId<'s>>,
    ta_id: Option<TAId>,
    title: Option<Title<'s>>,
    tp_id: Option<TPId>,
}

impl<'s> TryFrom<&'s str> for Ihr<'s> {
    type Error = anyhow::Error;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        let dict = split_key_values(value);
        let get = |key| dict.get(key).copied();

        Ok(Self {
            amg_artist_id: get("amgArtistId").map(AmgArtistId::try_from).transpose()?,
            amg_artwork_url: get("amgArtworkURL")
                .map(AmgArtworkUrl::try_from)
                .transpose()?,
            amg_track_id: get("amgTrackId").map(AmgTrackId::try_from).transpose()?,
            artist: get("artist").map(Artist),
            cartcut_id: get("cartcutId").map(CartCutId::try_from).transpose()?,
            itunes_track_id: get("itunesTrackId")
                .map(ITunesTrackId::try_from)
                .transpose()?,
            length: get("length").map(Length::try_from).transpose()?,
            media_base_id: get("MediaBaseId").map(MediaBaseId::try_from).transpose()?,
            song_spot: get("song_spot").map(SongSpot::try_from).transpose()?,
            sp_event_id: get("spEventID").map(SpEventId::try_from).transpose()?,
            spot_instance_id: get("spotInstanceId")
                .map(SpotInstanceId::try_from)
                .transpose()?,
            ta_id: get("TAID").map(TAId::try_from).transpose()?,
            title: get("title").map(Title),
            tp_id: get("TPID").map(TAId::try_from).transpose()?,
        })
    }
}

impl<'s> Ihr<'s> {
    fn is_music(&self) -> bool {
        (self.song_spot.contains(&SongSpot::M) || self.song_spot.contains(&SongSpot::F))
            && self.length.map_or(false, |x| x.0 > Duration::from_secs(90))
            && (self.media_base_id.map_or(false, |x| x.is_valid())
                || self.itunes_track_id.map_or(false, |x| x.is_valid())
                || (self.amg_artist_id.map_or(false, |x| x.is_valid())
                    && self.amg_track_id.map_or(false, |x| x.is_valid()))
                || self.tp_id.map_or(false, |x| x.is_valid())
                || self.amg_artwork_url.map_or(false, |x| x.is_url()))
    }

    fn is_talk(&self) -> bool {
        self.song_spot.contains(&SongSpot::T)
            && self.media_base_id.contains(&Id::ZERO)
            && self.itunes_track_id.contains(&Id::ZERO)
            && self.amg_track_id.contains(&Id::ZERO)
            && self.amg_artist_id.contains(&Id::ZERO)
            && self.ta_id.contains(&Id::ZERO)
            && self.tp_id.contains(&Id::ZERO)
            && self.cartcut_id.contains(&Id::ZERO)
            && self.amg_artwork_url.is_none()
            && self.length.contains(&Length::ZERO)
            && self.spot_instance_id.contains(&IdOrUuid::INVALID)
    }

    fn is_advertisment(&self) -> bool {
        let a = self.song_spot.contains(&SongSpot::F)
            && self.media_base_id.contains(&Id::ZERO)
            && self.itunes_track_id.contains(&Id::ZERO)
            && self.amg_artist_id.contains(&Id::ZERO)
            && self.amg_track_id.contains(&Id::INVALID)
            && self.ta_id.contains(&Id::ZERO)
            && self.tp_id.contains(&Id::ZERO)
            && self.cartcut_id.contains(&Id::ZERO)
            && self.amg_artwork_url.map_or(false, |x| x.is_null())
            && self.spot_instance_id.map_or(false, |x| x.is_uuid());

        let b = self.artist.map_or(false, |x| !x.0.is_empty())
            && self.title.map_or(false, |x| !x.0.is_empty())
            && self.song_spot.contains(&SongSpot::T)
            && self.media_base_id.contains(&Id::ZERO)
            && self.itunes_track_id.contains(&Id::ZERO)
            && self.amg_track_id.contains(&Id::INVALID)
            && self.amg_artist_id.contains(&Id::ZERO)
            && self.amg_artwork_url.map_or(false, |x| x.is_null())
            && self.ta_id.contains(&Id::ZERO)
            && self.tp_id.contains(&Id::ZERO)
            && self.length.map_or(true, |x| x.0 < Duration::from_secs(65))
            && self.cartcut_id.map_or(false, |x| x.is_valid())
            && matches!(self.spot_instance_id, Some(IdOrUuid::Id(id)) if id.is_valid());
        a || b
    }

    fn guess_kind(&self, artist: Option<&str>, title: Option<&str>) -> Option<ContentKind> {
        let mismatch = |comment: &Option<&str>, tag: &Option<&str>| match (comment, tag) {
            (Some(comment), Some(tag)) if comment != tag => {
                log::error!("Value mismatch: comment={comment}, tag={tag}");
                true
            }
            _ => false,
        };

        if mismatch(&artist, &self.artist.map(|x| x.0))
            || mismatch(&title, &self.title.map(|x| x.0))
        {
            // Values of TrackArtist/TrackTitle do not match the artist/title values from the comment tag.
            // Skip it now, I will do something better later.
            return None;
        }

        if self.is_advertisment() {
            Some(ContentKind::Advertisement)
        } else if self.is_music() {
            Some(ContentKind::Music)
        } else if self.is_talk() {
            Some(ContentKind::Talk)
        } else {
            None
        }
    }
}

fn split_key_values(s: &str) -> HashMap<&str, &str> {
    lazy_static! {
        static ref RE_KEY_VALUE: Regex = Regex::new(
            r#"(\w+)=("([^=]+?)"|\\"([^=]+?)\\"|\\\\"([^=]+?)\\\\"|\\\\\\"([^=]+?)\\\\\\")"#
        )
        .unwrap();
    }

    RE_KEY_VALUE
        .captures_iter(s)
        .filter_map(|cap| {
            let key = cap.get(1);
            let value = cap
                .get(3)
                .or_else(|| cap.get(4))
                .or_else(|| cap.get(5))
                .or_else(|| cap.get(6));
            key.zip(value).map(|(k, v)| (k.as_str(), v.as_str()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use model::ContentKind;

    use super::split_key_values;
    use super::AmgArtworkUrl;
    use super::Artist;
    use super::Id;
    use super::IdOrUuid;
    use super::Ihr;
    use super::Length;
    use super::SongSpot;
    use super::Title;

    const MUSIC: &[&str] = &[
        r#"song_spot="F" MediaBaseId="0" itunesTrackId="0" amgTrackId="-1" amgArtistId="0" TAID="0" TPID="44166046" cartcutId="0" amgArtworkURL="http://image.iheart.com/ihr-ingestion-pipeline-production-sbmg/incoming/prod/DDEX/A10301A0003197934N_20170622015918436/resources/A10301A0003197934N_T-1020218987_Image.jpg" length="00:03:32" unsID="-1" spotInstanceId="54d8c36d-b3d0-45f3-8bce-4ff376766e1e""#,
        r#"title=\"Spiders\",artist=\"System Of A Down\",url=\"song_spot=\\\"M\\\" spotInstanceId=\\\"-1\\\" length=\\\"00:03:33\\\" MediaBaseId=\\\"1187579\\\" TAID=\\\"0\\\" TPID=\\\"8795354\\\" cartcutId=\\\"744953\\\" amgArtworkURL=\\\"http://image.iheart.com/ihr-ingestion-pipeline-production-sbmg/A10301A0000935626B_20180529180357702/495735.20126.jpg\\\" spEventID=\\\"6f404c81-6435-ed11-aab9-025576f7aad7\\\" \""#,
    ];

    const ADS: &[&str] = &[
        r#"title=\"9000778 Main Ac 3\",artist=\"9000778 Main Ac 3\",url=\"song_spot=\\\"T\\\" MediaBaseId=\\\"0\\\" itunesTrackId=\\\"0\\\" amgTrackId=\\\"-1\\\" amgArtistId=\\\"0\\\" TAID=\\\"0\\\" TPID=\\\"0\\\" cartcutId=\\\"9000778001\\\" amgArtworkURL=\\\"null\\\" length=\\\"00:00:29\\\" unsID=\\\"-1\\\" spotInstanceId=\\\"97638027\\\""#,
        r#"title="Grocery Outlet",artist="Agency",url="song_spot=\"T\" MediaBaseId=\"0\" itunesTrackId=\"0\" amgTrackId=\"-1\" amgArtistId=\"0\" TAID=\"0\" TPID=\"0\" cartcutId=\"7808299001\" amgArtworkURL=\"null\" length=\"00:00:29\" unsID=\"-1\" spotInstanceId=\"94695128\""#,
        r#"song_spot="F" MediaBaseId="0" itunesTrackId="0" amgTrackId=\"-1\" amgArtistId=\"0\" TAID=\"0\" TPID=\"0\" cartcutId=\"0\" amgArtworkURL=\"null\" length=\"00:02:03\" unsID=\"-1\" spotInstanceId=\"688d6785-f34c-35a8-3255-1a9dd167fbd2\""#,
    ];

    const TALK: &[&str] = &[
        r#"song_spot="T" MediaBaseId="0" itunesTrackId="0" amgTrackId="0" amgArtistId="0" TAID="0" TPID="0" cartcutId="0" amgArtworkURL="" length="00:00:00" unsID="0" spotInstanceId="-1""#,
    ];

    fn get_kind(value: &str) -> anyhow::Result<Option<ContentKind>> {
        Ihr::try_from(value).map(|ihr| {
            // println!("\n\n{value}\n{ihr:?}\n\n");
            ihr.guess_kind(None, None)
        })
    }

    #[test]
    fn test_parse_ihr() {
        let ihr = Ihr::try_from(MUSIC[1]).unwrap();
        assert!(ihr.artist.contains(&Artist("System Of A Down")));
        assert!(ihr.title.contains(&Title("Spiders")));
        assert!(ihr.song_spot.contains(&SongSpot::M));
        assert!(ihr.spot_instance_id.contains(&IdOrUuid::INVALID));
        assert!(ihr.length.contains(&Length(Duration::from_secs(213))));
        assert!(ihr.media_base_id.contains(&Id(1187579)));
        assert!(ihr.ta_id.contains(&Id::ZERO));
        assert!(ihr.tp_id.contains(&Id(8795354)));
        assert!(ihr.cartcut_id.contains(&Id(744953)));
        assert!(ihr.amg_artwork_url.contains(&AmgArtworkUrl::Url(
            "http://image.iheart.com/ihr-ingestion-pipeline-production-sbmg/A10301A0000935626B_20180529180357702/495735.20126.jpg"
        )));
        assert!(ihr
            .sp_event_id
            .contains(&IdOrUuid::uuid("6f404c81-6435-ed11-aab9-025576f7aad7")));
    }

    #[test]
    fn test_iheart_guesser_music() {
        assert_eq!(ContentKind::Music, get_kind(MUSIC[0]).unwrap().unwrap());
        assert_eq!(ContentKind::Music, get_kind(MUSIC[1]).unwrap().unwrap());
    }

    #[test]
    fn test_iheart_guesser_talk() {
        assert_eq!(ContentKind::Talk, get_kind(TALK[0]).unwrap().unwrap());
    }

    #[test]
    fn test_iheart_guesser_ads() {
        assert_eq!(
            ContentKind::Advertisement,
            get_kind(ADS[0]).unwrap().unwrap()
        );
        assert_eq!(
            ContentKind::Advertisement,
            get_kind(ADS[1]).unwrap().unwrap()
        );
        assert_eq!(
            ContentKind::Advertisement,
            get_kind(ADS[2]).unwrap().unwrap()
        );
    }

    #[test]
    fn test_split_key_value_empty() {
        assert!(split_key_values("").is_empty());
    }

    #[test]
    fn test_split_key_value_no_key_value() {
        assert!(split_key_values("key").is_empty());
        assert!(split_key_values("key=").is_empty());
        assert!(split_key_values("=value").is_empty());
        assert!(split_key_values("=").is_empty());
    }

    #[test]
    fn test_split_key_value_single_pair_unquoted() {
        assert!(split_key_values("key=value").is_empty());
    }

    #[test]
    fn test_split_key_value_single_pair() {
        assert_eq!(
            split_key_values(r#"key="value""#),
            [("key", "value")].into()
        );
        assert_eq!(
            split_key_values(r#"key=\"value\""#),
            [("key", "value")].into()
        );
        assert_eq!(
            split_key_values(r#"key=\\"value\\""#),
            [("key", "value")].into()
        );
        assert_eq!(
            split_key_values(r#"key=\\\"value\\\""#),
            [("key", "value")].into()
        );
    }

    #[test]
    fn test_split_key_value_multiple_pairs() {
        assert_eq!(
            split_key_values(
                r#"key="value",key2=\"value2\",key3=\\"value3\\" key4=\\\"value4\\\""#
            ),
            [
                ("key", "value"),
                ("key2", "value2"),
                ("key3", "value3"),
                ("key4", "value4"),
            ]
            .into()
        );
    }

    #[test]
    fn test_song_spot() {
        assert!(SongSpot::try_from("").is_err());
        assert!(SongSpot::try_from("A").is_err());
        assert!(SongSpot::try_from(" F").is_err());

        assert_eq!(SongSpot::try_from("F").unwrap(), SongSpot::F);
        assert_eq!(SongSpot::try_from("M").unwrap(), SongSpot::M);
        assert_eq!(SongSpot::try_from("T").unwrap(), SongSpot::T);
    }

    #[test]
    fn test_id() {
        assert!(Id::try_from("").is_err());
        assert!(Id::try_from("A").is_err());
        assert!(Id::try_from(" 12").is_err());

        assert_eq!(Id::try_from("0").unwrap(), Id(0));
        assert_eq!(Id::try_from("-1").unwrap(), Id(-1));
        assert_eq!(Id::try_from("12345").unwrap(), Id(12345));
    }
}
