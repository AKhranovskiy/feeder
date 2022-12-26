use std::collections::BTreeMap;
use std::fs::read_dir;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Error, Result};
use clap::{Parser, ValueEnum};
use futures::future::try_join_all;
use futures::TryFutureExt;
use kdam::{tqdm, BarExt};
use mongodb::bson::{doc, DateTime, Uuid};
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tags::Tags;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

const MONGO: &str = "mongodb://localhost:27017/?directConnection=true";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    let files = collect_file_names(&args.source)?;
    println!("Found {} files in {}", files.len(), args.source.display());

    let client_options = ClientOptions::parse(MONGO).await?;
    let client = Client::with_options(client_options)?;
    client
        .database("feeder")
        .run_command(doc! {"ping": 1}, None)
        .await?;

    let db = client.database("feeder");

    let pb = Arc::new(Mutex::new(tqdm!(
        total = files.len(),
        force_refresh = true,
        desc = "Inserted",
        unit = "file"
    )));

    try_join_all(files.into_iter().map(|path| {
        let pb = pb.clone();
        insert(db.clone(), path, &args).and_then(|_| async move {
            pb.lock().await.update(1);
            Ok(())
        })
    }))
    .await?;

    Ok(())
}

async fn insert(db: Database, path: PathBuf, args: &Args) -> Result<()> {
    let audio_doc = AudioDocument::try_from(&path)?;
    let metadata_doc = {
        let mut doc = MetadataDocument::try_from(&audio_doc)?;
        doc.kind = args.kind.clone();
        doc
    };

    if args.dry_run {
        println!("Inserting {:?} {}", args.kind, path.display());
        sleep(Duration::from_millis(300)).await;
    } else {
        db.collection("audio").insert_one(audio_doc, None).await?;
        db.collection("metadata")
            .insert_one(metadata_doc, None)
            .await?;
    }

    Ok(())
}

#[derive(Debug, Serialize)]
#[serde_as]
struct AudioDocument {
    pub id: Uuid,
    #[serde_as(as = "Bytes")]
    pub content: Vec<u8>,
    pub r#type: String,
}

impl TryFrom<&PathBuf> for AudioDocument {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        Ok(AudioDocument {
            id: Uuid::new(),
            content: std::fs::read(path)?,
            r#type: "audio/mpeg".into(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct MetadataDocument {
    pub id: Uuid,
    pub date_time: DateTime,
    pub kind: ContentKind,
    pub artist: String,
    pub title: String,
    // Must be BTreeMap because it is stored in DB.
    // Changing type would require wiping all records.
    pub tags: BTreeMap<String, String>,
}

impl TryFrom<&AudioDocument> for MetadataDocument {
    type Error = Error;

    fn try_from(doc: &AudioDocument) -> Result<Self, Self::Error> {
        let tags = Tags::try_from(doc.content.as_slice())?;

        Ok(Self {
            id: doc.id,
            date_time: DateTime::now(),
            kind: ContentKind::Advertisement,
            artist: tags.track_artist_or_empty(),
            title: tags.track_title_or_empty(),
            tags: tags.into(),
        })
    }
}

fn collect_file_names(path: &PathBuf) -> Result<Vec<PathBuf>> {
    Ok(read_dir(path)?
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        Some(entry.path())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<PathBuf>>())
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    dry_run: bool,

    source: PathBuf,

    #[arg(value_enum)]
    kind: ContentKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum ContentKind {
    Advertisement,
    Music,
    Talk,
}
