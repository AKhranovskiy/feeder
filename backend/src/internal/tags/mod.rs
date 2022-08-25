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
    &SpotBlock,
    &iheartradio::IHeartRadioGuesser,
];

// TODO - What to do if several guessers return a kind?
pub fn guess_content_kind(tags: &Tags) -> ContentKind {
    let kind = CONTENT_KIND_GUESSERS
        .iter()
        .fold(None, |kind, guesser| kind.or_else(|| guesser.guess(tags)))
        .unwrap_or(ContentKind::Unknown);

    if kind == ContentKind::Unknown {
        log::error!("Failed to guess content kind:\n{tags:#?}\n");
    }

    kind
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

struct SpotBlock;
impl ContentKindGuesser for SpotBlock {
    fn guess(&self, tags: &Tags) -> Option<ContentKind> {
        let comment = get_tag(tags, "Comment").unwrap_or_default();
        let artist = get_tag(tags, "TrackArtist").unwrap_or_default();
        let title = get_tag(tags, "TrackTitle").unwrap_or_default();

        if comment.contains(r#"text=\"Spot Block\" amgTrackId="#) {
            // TODO - check also length. it is less than 1 minute for ads and more than 3 mins for songs.
            if title.contains(r#"text="Spot Block" amgTrackId="#) {
                return Some(ContentKind::Advertisement);
            } else if !artist.is_empty() && !title.is_empty() {
                return Some(ContentKind::Music);
            }
        }

        None
    }
}
