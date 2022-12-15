use anyhow::Context;
use async_stream::try_stream;
use futures::{Stream, StreamExt, TryStreamExt};
use mongodb::bson::{doc, from_document, Document};
use mongodb::{Client, Collection};
use serde::Deserialize;
use serde_with::{serde_as, Bytes};

pub(crate) fn fetch_audio_content_stream(
    collection: Collection<Document>,
    kind: &str,
    count: u64,
) -> impl Stream<Item = anyhow::Result<Vec<u8>>> + Unpin {
    let pipeline = [
        doc! {"$match": doc! {"kind": kind}},
        doc! {"$sample": doc! {"size": count as i64}},
        doc! {"$lookup": doc! {
            "from": "audio",
            "localField": "id",
            "foreignField": "id",
            "as": "audio"
        }},
        doc! {"$unwind": doc! {"path": "$audio"}},
        doc! {"$replaceRoot": doc! {"newRoot": "$audio"}},
        doc! {"$project": doc! {"content": 1}},
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

fn extract_content(doc: Result<Document, mongodb::error::Error>) -> anyhow::Result<Vec<u8>> {
    let content = from_document::<AudioContentDocument>(doc?)?.content;
    Ok(content)
}

#[serde_as]
#[derive(Debug, Deserialize)]
struct AudioContentDocument {
    #[serde_as(as = "Bytes")]
    pub content: Vec<u8>,
}
#[cfg(not(feature = "small-data"))]
pub(super) async fn count_data(client: &Client) -> anyhow::Result<u64> {
    let db = client.database("feeder");
    let metadata = db.collection::<Document>("metadata");

    let (ads, music) = tokio::try_join!(
        metadata.count_documents(doc! {"kind": "Advertisement"}, None),
        metadata.count_documents(doc! {"kind": "Music"}, None),
    )?;

    Ok(ads.min(music))
}

#[cfg(feature = "small-data")]
pub(super) async fn count_data(_: &Client) -> anyhow::Result<u64> {
    Ok(1)
}

