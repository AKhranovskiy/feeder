use model::{ContentKind, Tags};

use crate::TagAnalyser;

pub struct Ttwn;

impl TagAnalyser for Ttwn {
    fn analyse(&self, tags: &Tags) -> ContentKind {
        match (tags.track_artist(), tags.track_title()) {
            (Some(artist), Some(title)) if artist == title && title.ends_with(" Ttwn") => {
                ContentKind::Advertisement
            }
            _ => ContentKind::Unknown,
        }
    }
}
