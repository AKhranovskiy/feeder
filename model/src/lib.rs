use std::collections::BTreeMap;
use std::time::Duration;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct Segment {
    pub url: Url,
    pub duration: Duration,
    #[serde(skip)]
    pub content: Option<Bytes>,
    pub tags: Tags,
}

pub type Tags = BTreeMap<String, String>;
