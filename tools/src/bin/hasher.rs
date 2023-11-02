use std::{
    ffi::OsStr,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use clap::Parser;
use kdam::BarExt;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, types::chrono::Utc, Sqlite};
use tokio::{sync::Mutex, task::JoinSet};

const WORKERS: usize = 10;
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MiB
const QUEUE_LIMIT: usize = 10;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let db = init_db(&cli.sqlite).await?;

    let input = Path::new(&cli.input);

    let (task_sender, task_receiver) = flume::bounded::<Task>(QUEUE_LIMIT);

    let pb = Arc::new(Mutex::new(
        kdam::BarBuilder::default()
            .unit("files")
            .desc("Processed")
            .force_refresh(true)
            .build()
            .map_err(|s| anyhow!(s))?,
    ));

    let mut workers = JoinSet::new();

    for _ in 0..WORKERS {
        let receiver = task_receiver.clone();
        let db = db.clone();
        let pb = pb.clone();

        workers.spawn(async move {
            let yamnet = match classifier::Yamnet::load("./models") {
                Ok(yamnet) => yamnet,
                Err(err) => {
                    panic!("Failed to load yamnet: {err}");
                }
            };

            while let Ok(task) = receiver.recv_async().await {
                let name = task.name.clone();

                match process(&yamnet, task) {
                    Ok(task) => match store(&db, task).await {
                        Ok(()) => {
                            if let Err(err) = pb.lock().await.update(1) {
                                eprintln!(
                                    "Failed to update progress bar for task={name}, err={err}",
                                );
                            }
                        }
                        Err(err) => {
                            eprintln!("Failed to store task={name}: {err}");
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to process task={name}: {error}");
                    }
                }
            }

            anyhow::Ok(())
        });
    }

    if input.is_file() && input.extension() == Some(OsStr::new("tar")) {
        println!("Processing tar-archive '{}'", input.display());

        let mut archive = tar::Archive::new(File::open(input)?);
        for entry in archive.entries()? {
            let entry = entry?;
            let path = entry.path()?.into_owned();
            let size = entry.size();

            if size == 0 {
                // Directory root
                continue;
            }

            if size as usize > MAX_FILE_SIZE {
                println!(
                    "File '{}' is too large, skipping, size: {}, max: {}",
                    path.display(),
                    size,
                    MAX_FILE_SIZE
                );

                continue;
            }

            let mut content = Vec::with_capacity(size as usize);
            entry.take(size).read_to_end(&mut content)?;

            let hash = seahash::hash(&content);

            if check_if_file_present(&db, hash).await? {
                // println!("File already present in dataset, file='{}'", path.display());
                continue;
            }

            task_sender.send(Task {
                name: path.display().to_string(),
                hash,
                content,
                kind: cli.kind,
                embedding: vec![],
                duration: Duration::default(),
            })?;
        }
    } else {
        eprintln!("Input '{}' is not a tar-archive", input.display());
    }

    println!("Finishing processing queue: {}", task_sender.len());
    while !task_sender.is_empty() {
        tokio::task::yield_now().await;
    }

    println!("Drop the task sender");
    drop(task_sender);

    println!("Join processing workers");
    while let Some(item) = workers.join_next().await {
        item??;
    }

    // println!("Finishing storing queue: {}", store_sender.len());
    // while !store_sender.is_empty() {
    //     tokio::task::yield_now().await;
    // }

    // println!("Drop the store sender");
    // drop(store_sender);

    // println!("Join storing worker");
    // store_worker.await??;

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input source. Supported sources: tar-archive, directory.
    #[arg(short, long)]
    input: PathBuf,

    /// Sqlite file
    #[arg(short, long)]
    sqlite: String,

    /// Dataset kind
    #[arg(short, long)]
    kind: DatasetKind,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, strum::AsRefStr)]
enum DatasetKind {
    Advert,
    Music,
    Other,
}

async fn init_db(path: &str) -> anyhow::Result<SqlitePool> {
    println!("Initializing Sqlite DB at '{path}'");

    let url = format!("sqlite://{path}");

    if !Sqlite::database_exists(&url).await? {
        println!("Creating database '{path}'");
        Sqlite::create_database(&url).await?;
    }

    let pool = SqlitePool::connect(&url).await?;

    sqlx::query(
        r"CREATE TABLE IF NOT EXISTS dataset (
        hash INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        content BLOB NOT NULL,
        kind TEXT NOT NULL,
        duration INTEGER NOT NULL,
        embedding BLOB NOT NULL,
        added TEXT NOT NULL
    )",
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

async fn check_if_file_present(db: &SqlitePool, hash: u64) -> anyhow::Result<bool> {
    #[allow(clippy::cast_possible_wrap)]
    let hash = hash as i64;

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dataset WHERE hash = ?")
        .bind(hash)
        .fetch_one(db)
        .await?;

    Ok(count > 0)
}

#[derive(Debug, Clone)]
struct Task {
    name: String,
    hash: u64,
    content: Vec<u8>,
    kind: DatasetKind,
    embedding: Vec<f32>,
    duration: std::time::Duration,
}

fn process(yamnet: &classifier::Yamnet, task: Task) -> anyhow::Result<Task> {
    let wav = codec::resample_16k_mono_s16_stream(task.content.as_slice())?;

    let duration = std::time::Duration::from_secs_f32(wav.len() as f32 / 16_000.0);

    let samples = wav.into_iter().map(f32::from).collect::<Vec<_>>();

    let data = classifier::Data::from_shape_vec((samples.len(),), samples)?;
    // Normalize data to [-1., 1.]
    let data = data / 32768.0;

    let embedding = yamnet.embedding(&data)?;

    Ok(Task {
        embedding,
        duration,
        ..task
    })
}

async fn store(db: &SqlitePool, task: Task) -> anyhow::Result<()> {
    #[allow(clippy::cast_possible_wrap)]
    sqlx::query(
        r"INSERT INTO dataset (hash, name, content, kind, duration, embedding, added) VALUES(?,?,?,?,?,?,?)",
    )
    .bind(task.hash as i64)
    .bind(&task.name)
    .bind(&task.content)
    .bind(task.kind.as_ref())
    .bind(task.duration.as_millis() as i64)
    .bind(bytemuck::cast_slice(&task.embedding))
    .bind(Utc::now())
    .execute(db)
    .await?;
    Ok(())
}
