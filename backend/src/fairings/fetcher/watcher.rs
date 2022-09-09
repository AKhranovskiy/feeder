use anyhow::{anyhow, Context};
use flume::Sender;
use futures::StreamExt;
use mongodb::change_stream::event::OperationType;
use rocket::{Orbit, Rocket};
use rocket_db_pools::Database;
use tokio::select;

use crate::internal::storage::Storage;
use crate::storage::streams::StreamDocument;

use super::StreamEvent;

const TARGET: &str = "StreamFetcher::Watcher";

pub(crate) async fn start_watcher(
    rocket: &Rocket<Orbit>,
    event_sender: Sender<StreamEvent>,
) -> anyhow::Result<()> {
    let storage = Storage::fetch(rocket).ok_or_else(|| anyhow!("Failed to acquire storage"))?;

    // TODO - extend StreamCollection interface.
    let mut changes = storage
        .database("feeder")
        .collection::<StreamDocument>("streams")
        .watch(None, None)
        .await
        .context("Subscribing for updates")?;

    let mut shutdown = rocket.shutdown();

    tokio::spawn(async move {
        loop {
            let change = select! {
                Some(change) = changes.next() => change,
                _ = &mut shutdown => {
                    log::info!(target: TARGET, "Shutting down");
                    break;
                },
                else => break,
            };

            let change = match change {
                Ok(ref change) => change,
                Err(ref error) => {
                    log::error!(target: TARGET, "Error: {error:#?}");
                    break;
                }
            };

            match change.operation_type {
                OperationType::Insert => {
                    let doc = change
                        .full_document
                        .clone()
                        .expect("Event contains full document");

                    if let Err(ref error) = event_sender.send_async(StreamEvent::Add(doc)).await {
                        log::error!(target: TARGET, "Failed to send Add event: {error:#?}");
                    }
                }
                OperationType::Delete => {
                    let id = change
                        .document_key
                        .clone()
                        .and_then(|doc| doc.get_object_id("_id").ok())
                        .expect("Event contains document key")
                        .to_hex();

                    if let Err(ref error) = event_sender.send_async(StreamEvent::Delete(id)).await {
                        log::error!(target: TARGET, "Failed to send Delete event: {error:#?}");
                    }
                }
                _ => {
                    log::error!(target: TARGET, "Unhandled event={change:#?}");
                }
            };
        }
    });

    Ok(())
}
