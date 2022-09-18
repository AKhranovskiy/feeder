use model::{ContentKind, Tags};

use crate::TagAnalyser;

pub struct SpotBlock;

impl TagAnalyser for SpotBlock {
    fn analyse(&self, tags: &Tags) -> ContentKind {
        let comment = tags.comment().unwrap_or_default();
        let artist = tags.track_artist().unwrap_or_default();
        let title = tags.track_title().unwrap_or_default();

        if comment.contains(r#"text=\"Spot Block\" amgTrackId="#) {
            // TODO - check also length. it is less than 1 minute for ads and more than 3 mins for songs.
            if title.contains(r#"text="Spot Block" amgTrackId="#) {
                ContentKind::Advertisement
            } else if !artist.is_empty() && !title.is_empty() {
                ContentKind::Music
            } else {
                ContentKind::Unknown
            }
        } else {
            ContentKind::Unknown
        }
    }
}
