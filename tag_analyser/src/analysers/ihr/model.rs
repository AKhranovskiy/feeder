use std::ops::Deref;
use std::str::FromStr;
use std::time::Duration;

use anyhow::anyhow;
use model::ContentKind;

use super::split::split_key_values;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SongSpot {
    F,
    M,
    T,
}

impl FromStr for SongSpot {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.chars().next() {
            Some('F') => Ok(Self::F),
            Some('M') => Ok(Self::M),
            Some('T') => Ok(Self::T),
            Some(x) => Err(anyhow!("unknown value={x}")),
            None => Err(anyhow!("empty value")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Artist<'str>(&'str str);

impl<'str> From<&'str str> for Artist<'str> {
    fn from(s: &'str str) -> Self {
        Self(s)
    }
}

impl Deref for Artist<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Title<'str>(&'str str);

impl<'str> From<&'str str> for Title<'str> {
    fn from(s: &'str str) -> Self {
        Self(s)
    }
}

impl Deref for Title<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(i64);

impl FromStr for Id {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = s.parse::<i64>().map(Self)?;
        Ok(id)
    }
}
impl Id {
    pub const INVALID: Id = Id(-1);
    pub const ZERO: Id = Id(0);

    pub fn is_valid(self) -> bool {
        self.0 > 0
    }
}

pub type AmgArtistId = Id;
pub type AmgTrackId = Id;
pub type CartCutId = Id;
pub type ITunesTrackId = Id;
pub type MediaBaseId = Id;
pub type TAId = Id;
pub type TPId = Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmgArtworkUrl<'str> {
    Null,
    Url(&'str str),
}

impl<'str> From<&'str str> for AmgArtworkUrl<'str> {
    fn from(s: &'str str) -> Self {
        match s {
            "null" => Self::Null,
            _ => Self::Url(s),
        }
    }
}

impl AmgArtworkUrl<'_> {
    pub fn is_null(&self) -> bool {
        matches!(self, &Self::Null)
    }

    pub fn is_url(&self) -> bool {
        matches!(self, &Self::Url(..))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Length(Duration);

impl FromStr for Length {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts = value
            .split(':')
            .take(3)
            .map(str::parse)
            .collect::<Result<Vec<u64>, _>>()?;

        match parts[..] {
            [h, m, s] => {
                let oh = h.checked_mul(3600);
                let om = m.checked_mul(60);
                oh.zip(om)
                    .and_then(|(hs, ms)| hs.checked_add(ms))
                    .and_then(|hms| hms.checked_add(s))
                    .map(Duration::from_secs)
                    .map(Self)
                    .ok_or_else(|| anyhow!("integer overflow: h={h}, m={m}, s={s}"))
            }
            _ => Err(anyhow!("expected `hh::mm::ss`, got {value}")),
        }
    }
}

impl Length {
    pub const ZERO: Length = Length(Duration::ZERO);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdOrUuid<'str> {
    Id(Id),
    Uuid(&'str str),
}

impl IdOrUuid<'_> {
    pub const INVALID: IdOrUuid<'static> = IdOrUuid::Id(Id::INVALID);

    pub fn is_uuid(&self) -> bool {
        matches!(self, &Self::Uuid(..))
    }
}

impl<'str> TryFrom<&'str str> for IdOrUuid<'str> {
    type Error = anyhow::Error;

    fn try_from(s: &'str str) -> Result<Self, Self::Error> {
        if s.len() < 12 {
            let id = Id::from_str(s).map(Self::Id)?;
            Ok(id)
        } else {
            let _ = uuid::Uuid::parse_str(s)?;
            Ok(Self::Uuid(s))
        }
    }
}

pub type SpotInstanceId<'str> = IdOrUuid<'str>;
pub type SpEventId<'str> = IdOrUuid<'str>;

#[derive(Debug)]
pub struct Ihr<'str> {
    pub amg_artist_id: Option<AmgArtistId>,
    pub amg_artwork_url: Option<AmgArtworkUrl<'str>>,
    pub amg_track_id: Option<AmgTrackId>,
    pub artist: Option<Artist<'str>>,
    pub cartcut_id: Option<CartCutId>,
    pub itunes_track_id: Option<ITunesTrackId>,
    pub length: Option<Length>,
    pub media_base_id: Option<MediaBaseId>,
    pub song_spot: Option<SongSpot>,
    pub sp_event_id: Option<SpEventId<'str>>,
    pub spot_instance_id: Option<SpotInstanceId<'str>>,
    pub ta_id: Option<TAId>,
    pub title: Option<Title<'str>>,
    pub tp_id: Option<TPId>,
}

impl<'str> TryFrom<&'str str> for Ihr<'str> {
    type Error = anyhow::Error;

    fn try_from(s: &'str str) -> Result<Self, Self::Error> {
        let dict = split_key_values(s);
        let get = |key| dict.get(key).copied();

        Ok(Self {
            amg_artist_id: get("amgArtistId").map(AmgArtistId::from_str).transpose()?,
            amg_artwork_url: get("amgArtworkURL").map(AmgArtworkUrl::from),
            amg_track_id: get("amgTrackId").map(AmgTrackId::from_str).transpose()?,
            artist: get("artist").map(Artist::from),
            cartcut_id: get("cartcutId").map(CartCutId::from_str).transpose()?,
            itunes_track_id: get("itunesTrackId")
                .map(ITunesTrackId::from_str)
                .transpose()?,
            length: get("length").map(Length::from_str).transpose()?,
            media_base_id: get("MediaBaseId").map(MediaBaseId::from_str).transpose()?,
            song_spot: get("song_spot").map(SongSpot::from_str).transpose()?,
            sp_event_id: get("spEventID").map(SpEventId::try_from).transpose()?,
            spot_instance_id: get("spotInstanceId")
                .map(SpotInstanceId::try_from)
                .transpose()?,
            ta_id: get("TAID").map(TAId::from_str).transpose()?,
            title: get("title").map(Title::from),
            tp_id: get("TPID").map(TAId::from_str).transpose()?,
        })
    }
}

impl Ihr<'_> {
    pub fn get_kind(&self) -> ContentKind {
        let ads = self.is_ads();
        let music = self.is_music();
        let talk = self.is_talk();

        match (ads, music, talk) {
            (true, false, false) => ContentKind::Advertisement,
            (false, true, false) => ContentKind::Music,
            (false, false, true) => ContentKind::Talk,
            (false, false, false) => ContentKind::Unknown,
            (a, m, t) => {
                log::error!(target: "TagAnalyser::IHR", "multiple kinds detected, ads={a}, music={m}, talk={t}");
                ContentKind::Unknown
            }
        }
    }

    fn is_ads(&self) -> bool {
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
            && self.cartcut_id.map_or(false, Id::is_valid)
            && matches!(self.spot_instance_id, Some(IdOrUuid::Id(id)) if id.is_valid());
        a || b
    }

    fn is_music(&self) -> bool {
        (self.song_spot.contains(&SongSpot::M) || self.song_spot.contains(&SongSpot::F))
            && self.length.map_or(false, |x| x.0 > Duration::from_secs(90))
            && (self.media_base_id.map_or(false, Id::is_valid)
                || self.itunes_track_id.map_or(false, Id::is_valid)
                || (self.amg_artist_id.map_or(false, Id::is_valid)
                    && self.amg_track_id.map_or(false, Id::is_valid))
                || self.tp_id.map_or(false, Id::is_valid)
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
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::analysers::ihr::test_data::{ADS, MUSIC, TALK};

    #[test]
    fn test_song_spot() {
        assert!(SongSpot::from_str("").is_err());
        assert!(SongSpot::from_str("A").is_err());
        assert!(SongSpot::from_str(" F").is_err());

        assert_eq!(SongSpot::from_str("F").unwrap(), SongSpot::F);
        assert_eq!(SongSpot::from_str("M").unwrap(), SongSpot::M);
        assert_eq!(SongSpot::from_str("T").unwrap(), SongSpot::T);
    }

    #[test]
    fn test_id() {
        assert!(Id::from_str("").is_err());
        assert!(Id::from_str("A").is_err());
        assert!(Id::from_str(" 12").is_err());

        assert_eq!(Id::from_str("0").unwrap(), Id(0));
        assert_eq!(Id::from_str("-1").unwrap(), Id(-1));
        assert_eq!(Id::from_str("12345").unwrap(), Id(12345));
    }

    #[test]
    fn test_parse_ihr() {
        let ihr = Ihr::try_from(MUSIC[1]).unwrap();
        assert!(ihr.artist.contains(&Artist("System Of A Down")));
        assert!(ihr.title.contains(&Title("Spiders")));
        assert!(ihr.song_spot.contains(&SongSpot::M));
        assert!(ihr.spot_instance_id.contains(&IdOrUuid::INVALID));
        assert!(ihr.length.contains(&Length(Duration::from_secs(213))));
        assert!(ihr.media_base_id.contains(&Id(1_187_579)));
        assert!(ihr.ta_id.contains(&Id::ZERO));
        assert!(ihr.tp_id.contains(&Id(8_795_354)));
        assert!(ihr.cartcut_id.contains(&Id(744_953)));
        assert!(ihr.amg_artwork_url.contains(&AmgArtworkUrl::Url(
            "http://image.iheart.com/ihr-ingestion-pipeline-production-sbmg/A10301A0000935626B_20180529180357702/495735.20126.jpg"
        )));
        assert!(ihr
            .sp_event_id
            .contains(&IdOrUuid::Uuid("6f404c81-6435-ed11-aab9-025576f7aad7")));
    }

    fn get_kind(value: &str) -> anyhow::Result<ContentKind> {
        Ihr::try_from(value).map(|ihr| {
            // println!("\n\n{value}\n{ihr:?}\n\n");
            ihr.get_kind()
        })
    }

    #[test]
    fn test_iheart_guesser_music() {
        assert_eq!(ContentKind::Music, get_kind(MUSIC[0]).unwrap());
        assert_eq!(ContentKind::Music, get_kind(MUSIC[1]).unwrap());
    }

    #[test]
    fn test_iheart_guesser_talk() {
        assert_eq!(ContentKind::Talk, get_kind(TALK[0]).unwrap());
    }

    #[test]
    fn test_iheart_guesser_ads() {
        assert_eq!(ContentKind::Advertisement, get_kind(ADS[0]).unwrap());
        assert_eq!(ContentKind::Advertisement, get_kind(ADS[1]).unwrap());
        assert_eq!(ContentKind::Advertisement, get_kind(ADS[2]).unwrap());
    }
}
