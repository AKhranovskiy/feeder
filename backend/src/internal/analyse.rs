use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use bytes::Bytes;
use futures::future::try_join3;
use futures::{StreamExt, TryStreamExt};
use mongodb::bson::doc;
use rocket_db_pools::Connection;
use serde::Serialize;

use model::{ContentKind, Segment, Tags};

use crate::internal::storage::MetadataDocument;

use super::emysound::find_matches;
use super::prediction::Prediction;
use super::storage::Storage;
use super::tags::extract_tags;
use super::{classification, guess_content_kind};

pub async fn analyse(
    storage: Connection<Storage>,
    content: &Bytes,
) -> anyhow::Result<(Tags, ContentKind, Vec<FingerprintMatch>, Vec<Prediction>)> {
    let ((tags, content_kind_from_tags), fingerpints, predictions) = try_join3(
        analyse_tags(content),
        lookup_fingerprints(storage, content),
        classify(content),
    )
    .await
    .map_err(|e| {
        log::error!("{e:#?}");
        e
    })?;

    Ok((tags, content_kind_from_tags, fingerpints, predictions))
}

async fn analyse_tags(content: &Bytes) -> anyhow::Result<(Tags, ContentKind)> {
    log::info!("Analyse tags");
    let tags = extract_tags(content)?;
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
    storage: Connection<Storage>,
    content: &Bytes,
) -> anyhow::Result<Vec<FingerprintMatch>> {
    log::info!("Lookup fingerprints");
    // TODO - find_matches should take a filename and content only. Even filename can be made up.
    let matches = find_matches(&Segment {
        url: url::Url::parse("https://localhost").unwrap(),
        duration: Duration::from_secs(0),
        content: content.clone(),
        content_type: "audio/mpeg".to_owned(),
        tags: Tags::new(),
    })
    .await?
    .unwrap_or_default();

    log::info!("{:?}", matches);

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
            doc.map(|doc| FingerprintMatch {
                id: from_bson_uuid(doc.id),
                artist: doc
                    .tags
                    .get(&"TrackArtist".to_owned())
                    .cloned()
                    .unwrap_or_default(),
                title: doc
                    .tags
                    .get(&"TrackTitle".to_owned())
                    .cloned()
                    .unwrap_or_default(),

                content_kind: doc.kind,
                score: scores
                    .get(&from_bson_uuid(doc.id))
                    .cloned()
                    .unwrap_or_default(),
            })
        })
        .try_collect::<Vec<_>>()
        .await?;

    Ok(metadata)
}

fn to_bson_uuid(uuid: uuid::Uuid) -> mongodb::bson::Uuid {
    mongodb::bson::Uuid::from_bytes(uuid.into_bytes())
}

fn from_bson_uuid(uuid: mongodb::bson::Uuid) -> uuid::Uuid {
    uuid::Uuid::from_bytes(uuid.bytes())
}

async fn classify(content: &Bytes) -> anyhow::Result<Vec<Prediction>> {
    log::info!("Classify");
    classification::classify(content, classification::AveragePerSecondScore)
}
