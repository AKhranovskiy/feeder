use model::{ContentKind, Tags};

use crate::TagAnalyser;

pub struct AdContext;

impl TagAnalyser for AdContext {
    fn analyse(&self, tags: &Tags) -> ContentKind {
        match tags.comment() {
            Some(comment) if comment.contains("adContext=") => ContentKind::Advertisement,
            _ => ContentKind::Unknown,
        }
    }
}
