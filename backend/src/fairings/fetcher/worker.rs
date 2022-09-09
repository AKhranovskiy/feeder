use flume::Receiver;
use futures::StreamExt;
use rocket::{Orbit, Rocket};
use tokio::select;

use crate::fairings::{RawSegmentPacket, RawSegmentTx};

use super::StreamEvent;

const TARGET: &str = "StreamFetcher::Worker";

pub(crate) async fn start_worker(
    rocket: &Rocket<Orbit>,
    events: Receiver<StreamEvent>,
) -> anyhow::Result<()> {
    let mut shutdown = rocket.shutdown();

    let raw_segments_tx = rocket
        .state::<RawSegmentTx>()
        .expect("RawSegment channel must be set up.")
        .0
        .clone();

    tokio::spawn(async move {
        let mut streams = tokio_stream::StreamMap::new();

        loop {
            select! {
                Ok(ev) = events.recv_async() => match ev {
                        StreamEvent::Add(ref stream) => {
                            log::info!(
                                target: TARGET,
                                "Stream added: id={}, name={}, url={}",
                                stream.id(),
                                stream.name,
                                stream.url
                            );

                            let segments = hls_fetcher::fetch(stream.url.to_string()).await;
                            streams.insert(stream.id(), segments);
                        },
                        StreamEvent::Delete(ref id) => {
                            log::info!(target: TARGET, "Stream deleted id={}", id,);
                            streams.remove(id);
                        }
                    },
                Some((id, result)) = streams.next() => match result{
                    Ok(segment) => if let Err(ref error) = raw_segments_tx.send_async(RawSegmentPacket::new(id.clone(), segment)).await {
                        log::info!(target: TARGET, "Stream {id} failed to push new segment: {error:#?}");
                    },
                    Err(ref error) => {log::error!("Stream {id}: {error}"); }
                },
                _ = &mut shutdown => {
                    log::info!(target: TARGET, "shutting down");
                    break;
                },
                else => break,
            };
        }
    });

    Ok(())
}
