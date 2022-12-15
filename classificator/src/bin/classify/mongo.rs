use anyhow::Context;
use async_stream::try_stream;
use futures::{Stream, StreamExt, TryStreamExt};
use mongodb::bson::{doc, from_document, Document};
use mongodb::{Collection};
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

