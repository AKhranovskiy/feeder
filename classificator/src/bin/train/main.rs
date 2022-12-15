use classificator::{mfcc, Classificator, Network};
use kdam::{tqdm, BarExt};

use futures::future::join_all;
use futures::{StreamExt, TryStreamExt};
use mongodb::bson::{doc, Document};
use mongodb::{options::ClientOptions, Client};

mod mongo;

const MONGO: &str = "mongodb://localhost:27017/?directConnection=true";

// TODO - Talks samples are garbage.
const KINDS: [&str; 2] = ["Advertisement", "Music" /*, "Talk"*/];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse your connection string into an options struct
    let client_options = ClientOptions::parse(MONGO).await?;

    // Get a handle to the cluster
    let client = Client::with_options(client_options)?;

    // Ping the server to see if you can connect to the cluster
    client
        .database("feeder")
        .run_command(doc! {"ping": 1}, None)
        .await?;

    let db = client.database("feeder");
    let metadata = db.collection::<Document>("metadata");

    let count = mongo::count_data(&client).await?;

    let data: Vec<Vec<f32>> = join_all(KINDS.iter().enumerate().map(|(pos, &kind)| {
        let count = count;
        let kind = kind;
        let metadata = metadata.clone();

        async move {
            println!("Fetch {kind} {count}");

            let mut pb = tqdm!(
                total = count as usize,
                desc = kind,
                position = pos as u16,
                force_refresh = true
            );

            mongo::fetch_audio_content_stream(metadata, kind, count)
                .and_then(decode)
                .and_then(|data| async move { mfcc::calculate(data.as_ref()).await })
                .inspect(|_| {pb.update(1);})
                .try_concat()
                .await
                .map(|data| {
                    pb.write(format!("Completed {kind}: {}kb", data.len() * 4 / 1024));
                    data
                })
        }
    }))
    .await
    .into_iter()
    .collect::<Result<_, _>>()?;

    let mut classificator = Classificator::empty(Network::CnnPp);
    classificator.batch_train(data).await?;

    Ok(())
}

async fn decode(data: Vec<u8>) -> anyhow::Result<Vec<f32>> {
    let output = classificator::decode::audio_to_pcm_s16le(data)
        .await?
        .into_iter()
        .map(f32::from)
        .collect();

    Ok(output)
}
