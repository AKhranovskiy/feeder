use std::collections::HashMap;
use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Tags {
    pub(crate) tags: HashMap<String, String>,
}

impl Display for Tags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.write_str("Tags [\n")?;
            f.write_str(
                &self
                    .tags
                    .iter()
                    .map(|(key, value)| format!("\t{key}: {value}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            )?;
            f.write_str("\n     ]")
        } else {
            f.write_str("Tags [")?;
            f.write_str(
                &self
                    .tags
                    .iter()
                    .map(|(key, value)| format!("({key}, {value})"))
                    .collect::<Vec<_>>()
                    .join(", "),
            )?;
            f.write_str("]")
        }
    }
}

impl Tags {
    #[must_use]
    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.tags.get(name).map(std::string::String::as_str)
    }

    pub fn get_or_empty(&self, name: &str) -> String {
        self.get(name).map(ToString::to_string).unwrap_or_default()
    }

    #[must_use]
    pub fn track_artist(&self) -> Option<&str> {
        self.get("TrackArtist")
    }
    #[must_use]
    pub fn track_artist_or_empty(&self) -> String {
        self.get_or_empty("TrackArtist")
    }

    #[must_use]
    pub fn track_title(&self) -> Option<&str> {
        self.get("TrackTitle")
    }
    #[must_use]
    pub fn track_title_or_empty(&self) -> String {
        self.get_or_empty("TrackTitle")
    }

    #[must_use]
    pub fn comment(&self) -> Option<&str> {
        self.get("Comment")
    }
    #[must_use]
    pub fn comment_or_empty(&self) -> String {
        self.get_or_empty("Comment")
    }

    #[must_use]
    pub fn with_comment(self, comment: &str) -> Self {
        let mut tags = self.tags;
        tags.insert("Comment".to_string(), comment.to_string());
        Self { tags }
    }

    #[must_use]
    pub fn txxx(&self) -> Option<&str> {
        self.get("TXXX")
    }
    #[must_use]
    pub fn wxxx(&self) -> Option<&str> {
        self.get("WXXX")
    }
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        self.get("URL")
    }

    #[must_use]
    pub fn album_artist(&self) -> Option<&str> {
        self.get("AlbumArtist")
    }

    #[must_use]
    pub fn album_title(&self) -> Option<&str> {
        self.get("AlbumTitle")
    }
    #[must_use]
    pub fn genre(&self) -> Option<&str> {
        self.get("Genre")
    }
    #[must_use]
    pub fn track_number(&self) -> Option<&str> {
        self.get("TrackNumber")
    }
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.get("FileName")
    }
}
