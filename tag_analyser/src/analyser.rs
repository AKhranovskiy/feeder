use model::{ContentKind, Tags};

pub trait TagAnalyser {
    fn analyse(&self, tags: &Tags) -> ContentKind;
}
