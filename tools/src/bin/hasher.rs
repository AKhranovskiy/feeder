use std::{
    ffi::OsStr,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

use clap::Parser;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, types::chrono::Utc, Sqlite};
use tokio::task::JoinSet;

const WORKERS: usize = 10;
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MiB

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let db = init_db(&cli.sqlite).await?;

    let input = Path::new(&cli.input);

    let (sender, receiver) = flume::bounded::<Task>(WORKERS);

    let mut workers = JoinSet::new();
    for i in 0..WORKERS {
        let receiver = receiver.clone();
        let db = db.clone();
        workers.spawn(async move {
            let yamnet = classifier::Yamnet::load("./models").unwrap();

            while let Ok(task) = receiver.recv() {
                println!(
                    "Worker {i}: processing '{}' {} {}",
                    task.name,
                    task.hash,
                    task.content.len()
                );
                let instant = Instant::now();
                process(&db, &yamnet, task).await;
                println!(
                    "Worker {i}: finished in {}ms",
                    instant.elapsed().as_millis()
                );
            }
            println!("Worker {i} completed");
        });
    }

    if input.is_file() && input.extension() == Some(OsStr::new("tar")) {
        println!("Processing tar-archive '{}'", input.display());

        let mut archive = tar::Archive::new(File::open(input)?);
        for entry in archive.entries()?.take(100) {
            let entry = entry?;
            let path = entry.path()?.into_owned();
            let size = entry.size();

            if size == 0 {
                // Directory root
                continue;
            }

            if size as usize > MAX_FILE_SIZE {
                eprintln!(
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
                println!("File already present in dataset, file='{}'", path.display());
                continue;
            }

            sender.send(Task {
                name: path.display().to_string(),
                hash,
                content,
                kind: cli.kind,
            })?;
        }
    } else {
        eprintln!("Input '{}' is not a tar-archive", input.display());
    }

    while !sender.is_empty() {
        tokio::task::yield_now().await;
    }

    // Close the channel.
    drop(sender);

    // Join all workers
    while let Some(item) = workers.join_next().await {
        let () = item.unwrap();
    }

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
}

async fn process(db: &SqlitePool, yamnet: &classifier::Yamnet, task: Task) {
    let wav = codec::resample_16k_mono_s16_stream(task.content.as_slice()).unwrap();
    let samples = wav.into_iter().map(f32::from).collect::<Vec<_>>();

    let data = classifier::Data::from_shape_vec((samples.len(),), samples).unwrap();
    // Normalize data to [-1., 1.]
    let data = data / 32768.0;

    let embedding = yamnet.embedding(&data).unwrap();

    let duration = codec::track_duration(task.content.as_slice()).unwrap();

    #[allow(clippy::cast_possible_wrap)]
    sqlx::query(
            r"INSERT INTO dataset (hash, content, kind, duration, embedding, added) VALUES(?,?,?,?,?,?)"
        ).bind(task.hash  as i64)
        .bind(&task.content)
        .bind(task.kind.as_ref())
        .bind(duration.as_millis() as i64)
        .bind(bytemuck::cast_slice(&embedding))
        .bind(Utc::now())
        .execute(db).await.unwrap();
}
