mod watcher;
mod worker;

use anyhow::{anyhow, Context};
use flume::Sender;
use futures::future::join_all;
use futures::{TryFutureExt, TryStreamExt};
use rocket::fairing;
use rocket::{Orbit, Rocket};
use rocket_db_pools::Database;

use crate::internal::storage::Storage;
use crate::storage::streams::{StreamDocument, StreamId};

use self::watcher::start_watcher;
use self::worker::start_worker;

pub struct Fetcher;

const STREAM_EVENT_BUFFER: usize = 10;

#[rocket::async_trait]
impl fairing::Fairing for Fetcher {
    fn info(&self) -> fairing::Info {
        use fairing::Kind;

        fairing::Info {
            name: "Stream Fetcher",
            kind: Kind::Liftoff | Kind::Singleton,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let (tx, rx) = flume::bounded(STREAM_EVENT_BUFFER);

        let result = start_worker(rocket, rx)
            .and_then(|_| load_streams(rocket, tx.clone()))
            .and_then(|_| start_watcher(rocket, tx.clone()))
            .await;

        if let Err(ref error) = result {
            log::error!(target: "StreamFetcher", "{error:#?}");
        }
    }
}

#[derive(Debug)]
pub(crate) enum StreamEvent {
    Add(StreamDocument),
    Delete(StreamId),
}

async fn load_streams(rocket: &Rocket<Orbit>, sender: Sender<StreamEvent>) -> anyhow::Result<()> {
    let storage = Storage::fetch(rocket).ok_or_else(|| anyhow!("Failed to acquire storage"))?;

    // TODO - extend StreamCollection api.
    let streams = storage
        .database("feeder")
        .collection::<StreamDocument>("streams")
        .find(None, None)
        .and_then(|cursor| cursor.try_collect::<Vec<_>>())
        .await
        .context("Reading streams")?;

    join_all(
        streams
            .into_iter()
            .map(StreamEvent::Add)
            .map(|ev| sender.send_async(ev)),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .context("Sending streams to worker")
    .map(|_| ())
}
