use std::sync::{Arc, Mutex};

use humansize::{FormatSize, BINARY};
use kdam::{tqdm, BarExt};

use futures::future::try_join_all;
use futures::{StreamExt, TryStreamExt};
use mongodb::bson::{doc, Document};
use mongodb::{options::ClientOptions, Client};
use rusqlite::{params, Connection};

use self::mongo::count_data;

mod mongo;

const MONGO: &str = "mongodb://localhost:27017/?directConnection=true";

const KINDS: [&str; 3] = ["Advertisement", "Music", "Talk"];

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

    let counts = count_data(&client, &KINDS).await?;

    // Prepare SQLite.
    let conn = Connection::open("./exported.sqlite")?;
    let init_query = r#"
        DROP TABLE IF EXISTS audio;
        CREATE TABLE audio (
            id BINARY(16) PRIMARY KEY,
            date TEXT NOT NULL,
            kind TEXT NOT NULL,
            artist TEXT,
            title TEXT,
            type TEXT NOT NULL,
            content BLOB NOT NULL
        ) WITHOUT ROWID;"#;

    conn.execute_batch(init_query)?;

    let conn = Arc::new(Mutex::new(conn));

    try_join_all((0..KINDS.len()).map(|i| {
        let kind = KINDS[i];
        let count = counts[i] as usize;
        let metadata = metadata.clone();
        let conn = conn.clone();

        async move {
            println!("Fetch {kind} {count}");

            let mut pb = tqdm!(
                total = count,
                desc = kind,
                position = i as u16,
                force_refresh = true
            );

            let total_size = mongo::fetch_audio_content_stream(metadata, kind)
                .inspect(|_| {
                    pb.update(1);
                })
                .and_then(|data| {
                    let conn = conn.clone();
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let conn = conn.lock().unwrap();
                            conn.execute(
                                r#"INSERT INTO audio VALUES(?, ?, ?, ?, ?, ?, ?)"#,
                                params![
                                    u128::from(ulid::Ulid::new()) as i128,
                                    time::OffsetDateTime::now_utc(),
                                    data.kind.as_str(),
                                    data.artist.as_str(),
                                    data.title.as_str(),
                                    data.r#type.as_str(),
                                    data.content
                                ],
                            )?;
                            Ok(data.content.len())
                        })
                        .await
                        .unwrap()
                    }
                })
                .try_collect::<Vec<_>>()
                .await?
                .into_iter()
                .sum::<usize>();

            pb.write(format!(
                "Completed {kind}: {} / {}",
                count,
                total_size.format_size(BINARY)
            ));
            anyhow::Ok(())
        }
    }))
    .await?;

    Ok(())
}
