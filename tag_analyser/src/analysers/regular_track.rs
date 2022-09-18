use model::{ContentKind, Tags};

use crate::TagAnalyser;

pub struct RegularTrack;

impl TagAnalyser for RegularTrack {
    fn analyse(&self, tags: &Tags) -> ContentKind {
        if tags.track_artist().is_some()
            && tags.track_title().is_some()
            && tags.album_title().is_some()
        {
            ContentKind::Music
        } else {
            ContentKind::Unknown
        }
    }
}
