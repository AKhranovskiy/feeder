use std::collections::BTreeMap;

use crate::Tags;

impl From<BTreeMap<String, String>> for Tags {
    fn from(btree: BTreeMap<String, String>) -> Self {
        Self {
            tags: btree.into_iter().collect(),
        }
    }
}

impl From<Tags> for BTreeMap<String, String> {
    fn from(tags: Tags) -> Self {
        tags.tags.into_iter().collect()
    }
}
