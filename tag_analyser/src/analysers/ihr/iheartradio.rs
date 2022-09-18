use itertools::Itertools;
use model::{ContentKind, Tags};

use crate::TagAnalyser;

use super::model::Ihr;

pub struct IHeartRadio;

impl TagAnalyser for IHeartRadio {
    fn analyse(&self, tags: &Tags) -> ContentKind {
        let artist = tags.track_artist();
        let title = tags.track_artist();

        let kinds = ["Comment", "TXXX", "URL", "WXXX"]
            .into_iter()
            .inspect(|s| log::debug!("Getting tag {s}"))
            .filter_map(|name| tags.get(name))
            .filter_map(|tag| match Ihr::try_from(tag) {
                Ok(ihr) => Some(ihr),
                Err(ref error) => {
                    log::error!(target: "TagAnalyser::IHR", "Failed to create IHR: {error:#}");
                    None
                }
            })
            .filter(|ihr| verify_tags(ihr, artist, title))
            .map(|ihr| ihr.get_kind())
            .unique()
            .collect_vec();

        if kinds.len() > 1 {
            log::error!("IHeartRadioGuesser detected multiple kinds: {kinds:?}");
        }

        match kinds[..] {
            [kind] => kind,
            _ => ContentKind::Unknown,
        }
    }
}

fn verify_tags(ihr: &Ihr, artist: Option<&str>, title: Option<&str>) -> bool {
    #[allow(clippy::shadow_reuse)]
    let matches = |comment: Option<&str>, tag: Option<&str>| match (comment, tag) {
        (Some(comment), Some(tag)) if comment != tag => {
            log::error!("Value mismatch: comment={comment}, tag={tag}");
            true
        }
        _ => false,
    };

    matches(artist, ihr.artist.as_deref()) && matches(title, ihr.title.as_deref())
}
