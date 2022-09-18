#![feature(option_result_contains)] // Ihr

mod analyser;
mod analysers;

use analyser::TagAnalyser;
use model::{ContentKind, Tags};

use self::analysers::TAG_ANALYSERS;

#[inline]
pub fn analyse_tags(tags: &Tags) -> ContentKind {
    TAG_ANALYSERS
        .iter()
        .fold(ContentKind::Unknown, |kind, analyser| {
            if kind == ContentKind::Unknown {
                analyser.analyse(tags)
            } else {
                kind
            }
        })
}
