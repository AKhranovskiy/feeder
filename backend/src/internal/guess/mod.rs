mod iheartradio;

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
        tags.comment().and_then(|comment| {
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
        if tags.track_artist().is_some()
            && tags.track_title().is_some()
            && tags.album_title().is_some()
        {
            Some(ContentKind::Music)
        } else {
            None
        }
    }
}

struct Ttwn;
impl ContentKindGuesser for Ttwn {
    fn guess(&self, tags: &Tags) -> Option<ContentKind> {
        tags.track_artist()
            .zip(tags.track_title())
            .filter(|(artist, title)| artist == title && title.ends_with(" Ttwn"))
            .map(|_| ContentKind::Advertisement)
    }
}

struct SpotBlock;
impl ContentKindGuesser for SpotBlock {
    fn guess(&self, tags: &Tags) -> Option<ContentKind> {
        let comment = tags.comment().unwrap_or_default();
        let artist = tags.track_artist().unwrap_or_default();
        let title = tags.track_title().unwrap_or_default();

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
