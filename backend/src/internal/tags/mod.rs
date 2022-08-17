mod extract;
mod iheartradio;

pub use extract::extract_tags;
use model::{ContentKind, Tags};

trait ContentKindGuesser {
    fn guess(&self, tags: &Tags) -> Option<ContentKind>;
}

const CONTENT_KIND_GUESSERS: &[&dyn ContentKindGuesser] = &[
    &AdContext,
    &RegularTrack,
    &Ttwn,
    &iheartradio::IHeartRadioGuesser,
];

// TODO - What to do if several guessers return a kind?
pub fn guess_content_kind(tags: &Tags) -> ContentKind {
    CONTENT_KIND_GUESSERS
        .iter()
        .fold(None, |kind, guesser| kind.or_else(|| guesser.guess(tags)))
        .unwrap_or(ContentKind::Unknown)
}

struct AdContext;

impl ContentKindGuesser for AdContext {
    fn guess(&self, tags: &Tags) -> Option<ContentKind> {
        tags.get(&"Comment".to_string()).and_then(|comment| {
            if comment.contains("adContext=") {
                Some(ContentKind::Advertisement)
            } else {
                None
            }
        })
    }
}

struct RegularTrack;

impl ContentKindGuesser for RegularTrack {
    fn guess(&self, tags: &Tags) -> Option<ContentKind> {
        if tag_not_empty(tags, "TrackArtist")
            && tag_not_empty(tags, "TrackTitle")
            && tag_not_empty(tags, "AlbumTitle")
        {
            Some(ContentKind::Music)
        } else {
            None
        }
    }
}

fn tag_not_empty(tags: &Tags, tag: &str) -> bool {
    get_tag(tags, tag).map_or(false, |value| !value.is_empty())
}

fn get_tag<'t>(tags: &'t Tags, name: &str) -> Option<&'t str> {
    tags.get(&name.to_string()).map(|value| value.as_str())
}

struct Ttwn;
impl ContentKindGuesser for Ttwn {
    fn guess(&self, tags: &Tags) -> Option<ContentKind> {
        if let Some(title) = get_tag(tags, "TrackTitle") &&
         let Some(artist) = get_tag(tags, "TrackArtist") &&
         title == artist && title.ends_with(" Ttwn") {
            Some(ContentKind::Advertisement)
        } else {
            None
        }
    }
}
