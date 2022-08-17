use std::time::Duration;

use anyhow::anyhow;
use lazy_static::lazy_static;

use model::{ContentKind, Tags};
use regex::Regex;

use super::get_tag;

pub struct IHeartRadioGuesser;

impl super::ContentKindGuesser for IHeartRadioGuesser {
    fn guess(&self, tags: &model::Tags) -> Option<ContentKind> {
        if is_promo_project(tags) {
            Some(ContentKind::Advertisement)
        } else if let Some(value) = get_tag(tags, "WXXX")
            .or_else(|| get_tag(tags, "TXXX"))
            .or_else(|| get_tag(tags, "Comment"))
        {
            IHeartRadioInfo::try_from(value)
                .map(|info| info.guess_kind())
                .map_err(|e| {
                    log::error!("IHeartRadioGuesser failed: Unrecongnised format, {e:#}\n{tags:#?}")
                })
                .ok()
        } else {
            None
        }
    }
}

fn is_promo_project(tags: &Tags) -> bool {
    get_tag(tags, "TrackTitle").map_or(false, |title| {
        title.contains("Iheart Promo Project")
            || title.contains("Ihm Promo Product")
            || title.starts_with("iHR ")
    })
}

#[derive(Debug)]
enum SpotInstanceId {
    Uuid(uuid::Uuid),
    Id(i64),
}

impl SpotInstanceId {
    fn is_valid(&self) -> bool {
        match self {
            SpotInstanceId::Uuid(_) => true,
            SpotInstanceId::Id(id) => id > &0,
        }
    }
}

impl TryFrom<&str> for SpotInstanceId {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Ok(uuid) = uuid::Uuid::parse_str(value) {
            Ok(SpotInstanceId::Uuid(uuid))
        } else if let Ok(id) = value.parse::<i64>() {
            Ok(SpotInstanceId::Id(id))
        } else {
            Err(anyhow!("Invalid SpotInstanceId: {}", value))
        }
    }
}

#[derive(Debug)]
struct IHeartRadioInfo {
    artist: Option<String>,
    title: Option<String>,
    song_spot: char,
    media_base_id: i64,
    itunes_track_id: i64,
    amg_track_id: i64,
    amg_artist_id: i64,
    ta_id: i64,
    tp_id: i64,
    cartcut_id: i64,
    amg_artwork_url: Option<reqwest::Url>,
    length: Duration,
    #[allow(dead_code)]
    uns_id: i64,
    spot_instance_id: Option<SpotInstanceId>,
}

impl TryFrom<&str> for IHeartRadioInfo {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r#"song_spot="(\w)" MediaBaseId="(-?\d+)" itunesTrackId="(-?\d+)" amgTrackId="(-?\d+)" amgArtistId="(-?\d+)" TAID="(-?\d+)" TPID="(-?\d+)" cartcutId="(-?\d+)" amgArtworkURL="(.*?)" length="(\d\d:\d\d:\d\d)" unsID="(-?\d+)" spotInstanceId="(.+?)""#).unwrap();
            static ref RE_ARTIST: Regex = Regex::new(r#"artist="([^"]+)"#).unwrap();
            static ref RE_TITLE: Regex = Regex::new(r#"title="([^"]+)"#).unwrap();
        }
        let unescaped = &value.replace(&r#"\""#, r#"""#);
        let unescaped = &unescaped.replace(&r#"\\"#, r#""#);

        let caps = RE
            .captures(unescaped)
            .ok_or_else(|| anyhow!("Failed to match iHeartRadio info\n{value}"))?;

        Ok(Self {
            artist: RE_ARTIST
                .captures(unescaped)
                .and_then(|cap| cap.get(0))
                .map(|s| s.as_str().to_owned()),
            title: RE_TITLE
                .captures(unescaped)
                .and_then(|cap| cap.get(0))
                .map(|s| s.as_str().to_owned()),
            song_spot: caps[1]
                .chars()
                .next()
                .ok_or_else(|| anyhow!("Failed to parse iHeartRadio::song_spot"))?,
            media_base_id: caps[2].parse::<i64>()?,
            itunes_track_id: caps[3].parse::<i64>()?,
            amg_track_id: caps[4].parse::<i64>()?,
            amg_artist_id: caps[5].parse::<i64>()?,
            ta_id: caps[6].parse::<i64>()?,
            tp_id: caps[7].parse::<i64>()?,
            cartcut_id: caps[8].parse::<i64>()?,
            amg_artwork_url: caps[9].to_owned().parse().ok(),
            length: parse_length(&caps[10])?,
            uns_id: caps[11].parse::<i64>()?,
            spot_instance_id: SpotInstanceId::try_from(&caps[12]).ok(),
        })
    }
}

fn parse_length(value: &str) -> anyhow::Result<Duration> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"(\d\d):(\d\d):(\d\d)"#).unwrap();
    }
    RE.captures(value)
        .ok_or_else(|| anyhow!("Expected hh:mm:ss, found {value}"))
        .and_then(|caps| {
            let h = caps[1].parse::<u64>()?;
            let m = caps[2].parse::<u64>()?;
            let s = caps[3].parse::<u64>()?;
            Ok(Duration::from_secs(h * 3600 + m * 60 + s))
        })
}

impl IHeartRadioInfo {
    fn is_music(&self) -> bool {
        (self.song_spot == 'M' || self.song_spot == 'F')
            && self.length > Duration::new(90, 0)
            && (self.media_base_id > 0
                || self.itunes_track_id > 0
                || (self.amg_artist_id > 0 && self.amg_track_id > 0)
                || (self.tp_id > 0)
                || self.amg_artwork_url.is_some())
    }

    fn is_talk(&self) -> bool {
        // song_spot=T MediaBaseId=0 itunesTrackId=0 amgTrackId=0 amgArtistId=0 TAID=0 TPID=0 cartcutId=0 amgArtworkURL="" length="00:00:00" unsID=0 spotInstanceId=-1
        self.song_spot == 'T'
            && self.media_base_id == 0
            && self.itunes_track_id == 0
            && self.amg_artist_id == 0
            && self.amg_track_id == 0
            && self.ta_id == 0
            && self.tp_id == 0
            && self.amg_artwork_url.is_none()
            && self
                .spot_instance_id
                .as_ref()
                .map_or(false, |id| !id.is_valid())
            && self.length == Duration::ZERO
    }

    fn is_advertisment(&self) -> bool {
        // song_spot=F MediaBaseId=0 itunesTrackId=0 amgTrackId=\"-1\" amgArtistId=\"0\" TAID=\"0\" TPID=\"0\" cartcutId=\"0\" amgArtworkURL=\"null\" length=\"00:02:03\" unsID=\"-1\" spotInstanceId=\"688d6785-f34c-35a8-3255-1a9dd167fbd2\""
        self.song_spot == 'F'
            && self.media_base_id == 0
            && self.itunes_track_id == 0
            && self.amg_artist_id == 0
            && self.amg_track_id == -1
            && self.ta_id == 0
            && self.tp_id == 0
            && self.cartcut_id == 0
            && self.amg_artwork_url.is_none()
            && self.spot_instance_id.as_ref().map_or(false, SpotInstanceId::is_valid) ||
// title=\"9000778 Main Ac 3\",artist=\"9000778 Main Ac 3\",url=\"song_spot=\\\"T\\\" MediaBaseId=\\\"0\\\" itunesTrackId=\\\"0\\\" amgTrackId=\\\"-1\\\" amgArtistId=\\\"0\\\" TAID=\\\"0\\\" TPID=\\\"0\\\" cartcutId=\\\"9000778001\\\" amgArtworkURL=\\\"null\\\" length=\\\"00:00:29\\\" unsID=\\\"-1\\\" spotInstanceId=\\\"97638027\\\""
            self.artist.as_ref()
                .zip(self.title.as_ref())
                .map_or(false, |(artist, title)| {!artist.is_empty() && artist == title})
                && self.song_spot == 'T'
                && self.media_base_id == 0
                && self.itunes_track_id == 0
                && self.amg_track_id == -1
                && self.amg_artist_id == 0
                && self.ta_id == 0
                && self.tp_id == 0
                && self.length >= Duration::from_secs(10)
                && self.cartcut_id > 0
                && self
                    .spot_instance_id.as_ref()
                    .map_or(false, |id| match id {
                        SpotInstanceId::Uuid(_) => true,
                        SpotInstanceId::Id(id) => id > &0,
                    })
    }

    fn guess_kind(&self) -> ContentKind {
        if self.is_advertisment() {
            ContentKind::Advertisement
        } else if self.is_music() {
            ContentKind::Music
        } else if self.is_talk() {
            ContentKind::Talk
        } else {
            ContentKind::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use model::ContentKind;

    use super::IHeartRadioInfo;

    const MUSIC: &str = r#"song_spot="F" MediaBaseId="0" itunesTrackId="0" amgTrackId="-1" amgArtistId="0" TAID="0" TPID="44166046" cartcutId="0" amgArtworkURL="http://image.iheart.com/ihr-ingestion-pipeline-production-sbmg/incoming/prod/DDEX/A10301A0003197934N_20170622015918436/resources/A10301A0003197934N_T-1020218987_Image.jpg" length="00:03:32" unsID="-1" spotInstanceId="54d8c36d-b3d0-45f3-8bce-4ff376766e1e""#;

    #[test]
    fn test_iheart_kind() {
        let info = IHeartRadioInfo::try_from(MUSIC).unwrap();
        assert_eq!(ContentKind::Music, info.guess_kind());
    }
}
