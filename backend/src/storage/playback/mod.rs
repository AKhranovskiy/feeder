use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Context};
use async_stream::stream;
use futures::stream::Stream;
use futures::{Future, StreamExt};
use model::ContentKind;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};
use mongodb::change_stream::event::{ChangeStreamEvent, OperationType};
use serde::{Deserialize, Serialize};
use tokio::select;

use super::StorageCollection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackDocument {
    #[serde(rename = "_id")]
    id: ObjectId,
    pub timestamp: bson::DateTime,
    pub stream_id: String,
    pub content_type: String,
    pub content: Vec<u8>,
    pub title: String,
    pub artist: String,
    pub duration_milliseconds: u32,
    pub classification: Vec<(ContentKind, f32)>,
}

pub type PlaybackCollection = StorageCollection<PlaybackDocument>;

#[derive(Debug)]
pub struct Playback {
    pub id: String,
    pub stream_id: String,
    pub content_type: String,
    pub content: Vec<u8>,
    pub title: String,
    pub artist: String,
    pub duration: Duration,
    pub classification: Vec<(ContentKind, f32)>,
}

impl Playback {
    pub fn new(
        stream_id: String,
        content_type: String,
        content: Vec<u8>,
        title: String,
        artist: String,
        duration: Duration,
        classification: Vec<(ContentKind, f32)>,
    ) -> Self {
        Self {
            id: bson::oid::ObjectId::new().to_hex(),
            stream_id,
            content_type,
            content,
            title,
            artist,
            duration,
            classification,
        }
    }
}

impl From<Playback> for PlaybackDocument {
    fn from(playback: Playback) -> Self {
        Self {
            id: ObjectId::parse_str(&playback.id).expect("Playback contains valid ObjectId"),
            timestamp: bson::DateTime::now(),
            stream_id: playback.stream_id,
            content_type: playback.content_type,
            content: playback.content,
            title: playback.title,
            artist: playback.artist,
            duration_milliseconds: playback.duration.as_millis() as u32,
            classification: playback.classification,
        }
    }
}

impl From<PlaybackDocument> for Playback {
    fn from(doc: PlaybackDocument) -> Self {
        Playback {
            id: doc.id.to_hex(),
            stream_id: doc.stream_id,
            content_type: doc.content_type,
            content: doc.content,
            title: doc.title,
            artist: doc.artist,
            duration: Duration::from_millis(doc.duration_milliseconds.into()),
            classification: doc.classification,
        }
    }
}

impl PlaybackCollection {
    pub async fn add(&self, playback: Playback) -> anyhow::Result<String> {
        let id = self
            .inner()
            .insert_one(PlaybackDocument::from(playback), None)
            .await
            .map(|result| {
                result
                    .inserted_id
                    .as_object_id()
                    .expect("Insert returns ObjectId")
                    .to_hex()
            })
            .context("Adding to storage")?;
        Ok(id)
    }

    #[allow(dead_code)]
    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Playback>> {
        let id = ObjectId::parse_str(id)?;
        let doc = self
            .inner()
            .find_one(doc! {"_id": id}, None)
            .await?
            .map(|doc| doc.into());
        Ok(doc)
    }

    pub async fn prune(&self, till: SystemTime) -> anyhow::Result<()> {
        let till = bson::DateTime::from_system_time(till);
        let filter = doc! {"timestamp": doc!{"$lt": till}};
        let _ = self.inner().delete_many(filter, None).await?;
        Ok(())
    }

    pub async fn watch(
        &self,
        mut stop: impl Future + Unpin,
    ) -> anyhow::Result<impl Stream<Item = PlaybackWatchEvent> + Unpin> {
        let mut changes = self
            .inner()
            .watch(None, None)
            .await
            .context("Watching playbacks")?;

        Ok(Box::pin(stream! {
            loop {
                let change = select! {
                    Some(change) = changes.next() => change,
                    _ = &mut stop => {
                        break;
                    },
                    else => {
                        yield PlaybackWatchEvent::Error(anyhow!("Unhandled loop event"));
                        break;
                    }
                };

                let change = match change {
                    Ok(change) => change,
                    Err(error) => {
                        yield PlaybackWatchEvent::Error(error.into());
                        break;
                    }
                };

                match change.operation_type {
                    OperationType::Insert => {
                        let id = get_hex_id(&change);
                        let playback: Playback = get_document(&change);
                        yield PlaybackWatchEvent::Add(id, playback);
                    },
                    OperationType::Delete => {
                        let id = get_hex_id(&change);
                        yield PlaybackWatchEvent::Delete(id);
                    }
                    _ => {
                        log::error!("Playback watcher: Unhandled event={change:#?}");
                        yield PlaybackWatchEvent::Error(anyhow!("Unhandled change, {change:#?}"));
                    }
                }
            }
        }))
    }
}

fn get_hex_id<D>(change: &ChangeStreamEvent<D>) -> String {
    change
        .document_key
        .as_ref()
        .and_then(|doc| doc.get_object_id("_id").ok())
        .expect("Event contains document key")
        .to_hex()
}

fn get_document<D, T>(change: &ChangeStreamEvent<D>) -> T
where
    T: From<D>,
    D: Clone,
{
    change
        .full_document
        .clone()
        .expect("Event contains full document")
        .into()
}

pub enum WatchEvent<Id, Item> {
    Add(Id, Item),
    Delete(Id),
    Error(anyhow::Error),
}

pub type PlaybackWatchEvent = WatchEvent<String, Playback>;
