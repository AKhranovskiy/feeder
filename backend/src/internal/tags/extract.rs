use std::io::Cursor;

use anyhow::Context;
use bytes::Bytes;
use model::Tags;

// TODO - move to separate lib.
pub fn extract_tags(content: &Bytes) -> anyhow::Result<Tags> {
    // log::error!(
    //     "Reading tags, content len: {}, bytes={}",
    //     content.len(),
    //     content[0..128]
    //         .iter()
    //         .map(|c| *c as char)
    //         .collect::<String>()
    // );
    use lofty::{ItemKey, ItemValue, Probe};

    let tagged_file = Probe::new(Cursor::new(content))
        .guess_file_type()
        .context("guessing file type")?
        .read(false)
        .context("reading tags");

    if let Err(e) = tagged_file {
        log::error!("Failed to read tags: {e:#?}");
        return Ok(Tags::new());
    }
    let tagged_file = tagged_file.unwrap();

    // TODO https://en.wikipedia.org/wiki/ID3
    let mut tags = Tags::new();
    for tag in tagged_file.tags() {
        for item in tag.items() {
            let key: Option<&str> = match item.key() {
                ItemKey::AlbumTitle => Some("AlbumTitle"),
                ItemKey::Comment => Some("Comment"),
                ItemKey::EncoderSettings => Some("EncoderSettings"),
                ItemKey::EncoderSoftware => Some("EncoderSoftware"),
                ItemKey::Genre => Some("Genre"),
                ItemKey::OriginalFileName => Some("FileName"),
                ItemKey::RecordingDate => Some("RecordingDate"),
                ItemKey::TrackArtist => Some("TrackArtist"),
                ItemKey::TrackNumber => Some("TrackNumber"),
                ItemKey::TrackTitle => Some("TrackTitle"),
                ItemKey::TrackTotal => Some("TrackTotal"),
                ItemKey::Unknown(v) => Some(v.as_str()),
                _ => {
                    log::error!(
                        "Unsupported tag: key={:?}, value={:?}",
                        item.key(),
                        item.value()
                    );
                    None
                }
            };
            if let Some(key) = key {
                let value = match item.value() {
                    ItemValue::Text(v) | ItemValue::Locator(v) => v.clone(),
                    ItemValue::Binary(v) => std::str::from_utf8(v)?.to_owned(),
                };

                // Ignore empty tags.
                let value = value.trim();
                if !value.is_empty() {
                    tags.insert(key.to_owned(), value.to_owned());
                } else {
                    log::info!("Tag {key} is empty. Skip.");
                }
            }
        }
    }
    Ok(tags)
}
