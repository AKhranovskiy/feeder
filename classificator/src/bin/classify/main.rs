use classificator::{Classificator, Network};

use futures::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::{options::ClientOptions, Client};

mod mongo;

const MONGO: &str = "mongodb://localhost:27017/?directConnection=true";

// TODO - Talks samples are garbage.
const KINDS: [&str; 2] = ["Advertisement", "Music" /*, "Talk"*/];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client_options = ClientOptions::parse(MONGO).await?;
    let client = Client::with_options(client_options)?;
    client
        .database("feeder")
        .run_command(doc! {"ping": 1}, None)
        .await?;

    let db = client.database("feeder");
    let metadata = db.collection::<Document>("metadata");


    let classificator = Classificator::empty(Network::CnnPp);

    let _: Vec<()> = mongo::fetch_audio_content_stream(metadata, KINDS[0], 1)
        .and_then(|data| classificator.classify(data))
        .try_collect()
        .await?;

    Ok(())
}
