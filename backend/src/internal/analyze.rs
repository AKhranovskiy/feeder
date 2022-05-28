use model::{ContentKind, Tags};

pub fn analyze(tags: &Tags) -> Option<ContentKind> {
    AdContextGuesser::guess(tags)
}

trait ContentKindGuesser {
    fn guess(tags: &Tags) -> Option<ContentKind>;
}

struct AdContextGuesser;

impl ContentKindGuesser for AdContextGuesser {
    fn guess(tags: &Tags) -> Option<ContentKind> {
        tags.get(&"Comment".to_string()).and_then(|comment| {
            if comment.contains("adContext=") {
                Some(ContentKind::Advertisement)
            } else {
                None
            }
        })
    }
}

struct IHeartGuesser;

impl ContentKindGuesser for IHeartGuesser {
    fn guess(tags: &Tags) -> Option<ContentKind> {
        if let Some(value) = tags
            .get(&"WXXX".to_string())
            .or_else(|| tags.get(&"TXXX".to_string()))
            .or_else(|| tags.get(&"Comment".to_string()))
        {
            log::debug!("IHeartGuesser::guess {value}")
        }
        None
    }
}
