use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use futures::future::try_join;
use futures::{StreamExt, TryStreamExt};
use mongodb::bson::doc;
use serde::Serialize;

use model::{ContentKind, Segment, Tags};

use crate::internal::storage::MetadataDocument;
use crate::internal::{from_bson_uuid, to_bson_uuid};

use super::emysound::find_matches;
use super::guess_content_kind;
use super::prediction::Prediction;

pub async fn analyse(
    storage: &mongodb::Client,
    content: &[u8],
    comment: &str,
) -> anyhow::Result<(Tags, ContentKind, Vec<FingerprintMatch>, Vec<Prediction>)> {
    let ((tags, content_kind_from_tags), fingerpints) = try_join(
        analyse_tags(content, comment),
        lookup_fingerprints(storage, content),
    )
    .await
    .context("Segment analyse")?;

    Ok((tags, content_kind_from_tags, fingerpints, vec![]))
}

pub async fn analyse_tags(content: &[u8], comment: &str) -> anyhow::Result<(Tags, ContentKind)> {
    let tags = Tags::try_from(content)
        .context("Tag analyse")?
        .with_comment(comment);
    let kind = guess_content_kind(&tags);
    Ok((tags, kind))
}

#[derive(Debug, Serialize)]
// TODO - Merge with SegmentMatchResponse.
pub struct FingerprintMatch {
    pub id: uuid::Uuid,
    pub artist: String,
    pub title: String,
    pub content_kind: ContentKind,
    pub score: f32,
}

async fn lookup_fingerprints(
    storage: &mongodb::Client,
    content: &[u8],
) -> anyhow::Result<Vec<FingerprintMatch>> {
    // TODO - find_matches should take a filename and content only. Even filename can be made up.
    let matches = find_matches(&Segment {
        url: String::default(),
        duration: Duration::from_secs(0),
        content: content.to_vec(),
        content_type: "audio/mpeg".to_owned(),
        tags: Tags::default(),
    })
    .await
    .context("Fingerprints lookup")?
    .unwrap_or_default();

    let ids = matches
        .iter()
        .map(|m| to_bson_uuid(m.id))
        .collect::<Vec<_>>();

    let scores = matches
        .iter()
        .map(|m| (m.id, m.score as f32 / 255.0))
        .collect::<HashMap<_, _>>();

    let metadata = storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata")
        .find(doc! {"id": doc!{"$in": ids}}, None)
        .await
        .context("Retrieving metadata")?
        .map(|doc| {
            doc.map(|doc| {
                let tags = Tags::from(doc.tags.clone());
                FingerprintMatch {
                    id: from_bson_uuid(doc.id),
                    artist: tags.track_artist_or_empty(),
                    title: tags.track_title_or_empty(),
                    content_kind: doc.kind,
                    score: scores
                        .get(&from_bson_uuid(doc.id))
                        .cloned()
                        .unwrap_or_default(),
                }
            })
        })
        .try_collect::<Vec<_>>()
        .await
        .context("Collecting fingerpints")?;

    Ok(metadata)
}
