use anyhow::Context;
use async_stream::try_stream;
use futures::future::try_join_all;
use futures::{Stream, StreamExt, TryStreamExt};
use mongodb::bson::{doc, from_document, Document};
use mongodb::{Client, Collection};
use serde::Deserialize;
use serde_with::{serde_as, Bytes};

pub(crate) fn fetch_audio_content_stream(
    collection: Collection<Document>,
    kind: &str,
) -> impl Stream<Item = anyhow::Result<AudioDataDocument>> + Unpin {
    let pipeline = [
        doc! {"$match": doc! {"kind": kind}},
        doc! {"$lookup": doc! {
            "from": "audio",
            "localField": "id",
            "foreignField": "id",
            "as": "audio"
        }},
        doc! {
            "$project": doc! {
                "artist": "$artist",
                "title": "$title",
                "kind": "$kind",
                "content": doc! {
                    "$arrayElemAt": [
                        "$audio",
                        0
                    ]
                }
            }
        },
        doc! {
            "$project": doc! {
                "artist": "$artist",
                "title": "$title",
                "kind": "$kind",
                "content": "$content.content",
                "type": "$content.type"
            }
        },
    ];

    Box::pin(try_stream! {
        let mut cursor = collection
            .aggregate(pipeline, None)
            .await
            .context("Aggregating")?
            .map(extract_content);

        while let Some(data) = cursor.try_next().await? {
            yield data
        }
    })
}

fn extract_content(
    doc: Result<Document, mongodb::error::Error>,
) -> anyhow::Result<AudioDataDocument> {
    let content = from_document::<AudioDataDocument>(doc?)?;
    Ok(content)
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct AudioDataDocument {
    pub artist: String,
    pub title: String,
    pub kind: String,
    pub r#type: String,
    #[serde_as(as = "Bytes")]
    pub content: Vec<u8>,
}

pub(super) async fn count_data(client: &Client, kinds: &[&str]) -> anyhow::Result<Vec<u64>> {
    let db = client.database("feeder");
    let metadata = db.collection::<Document>("metadata");

    let counts = try_join_all(kinds.iter().map(|kind| {
        let metadata = metadata.clone();
        async move { metadata.count_documents(doc! {"kind": kind}, None).await }
    }))
    .await?;

    Ok(counts)
}
