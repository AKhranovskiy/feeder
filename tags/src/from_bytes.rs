use std::io::Cursor;

use anyhow::{bail, Context};
use lofty::{ItemKey, ItemValue, Probe, TagItem, TaggedFileExt};

use crate::Tags;

// TryFrom does not do unsized coercion.
impl<const N: usize> TryFrom<&[u8; N]> for Tags {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8; N]) -> Result<Self, Self::Error> {
        // Just hand it over to the slice version.
        <Tags as TryFrom<&[u8]>>::try_from(bytes)
    }
}

impl TryFrom<&[u8]> for Tags {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let tagged_file = Probe::new(Cursor::new(bytes))
            .guess_file_type()
            .context("guessing file type")?
            .read()
            .context("reading tags");

        if let Err(e) = tagged_file {
            log::error!("Failed to read tags: {e:#?}");
            return Ok(Tags::default());
        }

        let tagged_file = tagged_file.unwrap();

        let (oks, errs): (Vec<_>, Vec<_>) = tagged_file
            .tags()
            .iter()
            .flat_map(|tag| tag.items().map(key_value))
            .filter_map(|v| v.transpose())
            .partition(Result::is_ok);

        let error: String = errs
            .into_iter()
            .map(Result::unwrap_err)
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        if !error.is_empty() {
            bail!(error)
        }

        Ok(Tags {
            tags: oks.into_iter().map(Result::unwrap).collect(),
        })
    }
}

// TODO there can be multiple values for the same key. they must be merged together.
fn key_value(tag: &TagItem) -> anyhow::Result<Option<(String, String)>> {
    let key = tag_key(tag)?;
    Ok(tag_value(tag).map(|value| (key, value)))
}

fn tag_key(tag: &TagItem) -> anyhow::Result<String> {
    let key = match tag.key() {
        ItemKey::AlbumTitle => "AlbumTitle",
        ItemKey::Comment => "Comment",
        ItemKey::EncoderSettings => "EncoderSettings",
        ItemKey::EncoderSoftware => "EncoderSoftware",
        ItemKey::Genre => "Genre",
        ItemKey::OriginalFileName => "FileName",
        ItemKey::RecordingDate => "RecordingDate",
        ItemKey::TrackArtist => "TrackArtist",
        ItemKey::TrackNumber => "TrackNumber",
        ItemKey::TrackTitle => "TrackTitle",
        ItemKey::TrackTotal => "TrackTotal",
        ItemKey::EncodedBy => "EncodedBy",
        ItemKey::Year => "Year",
        ItemKey::Unknown(v) => v.as_str(),
        not_supported => bail!("Not supported tag: key={not_supported:?}"),
    };
    Ok(key.to_string())
}

fn tag_value(tag: &TagItem) -> Option<String> {
    let value = match tag.value() {
        ItemValue::Text(v) | ItemValue::Locator(v) => v.clone(),
        ItemValue::Binary(v) => std::str::from_utf8(v)
            .map(|s| s.to_string())
            .unwrap_or_default(),
    };
    let value = value.trim();

    if !value.is_empty() {
        Some(value.to_string())
    } else {
        None
    }
}
