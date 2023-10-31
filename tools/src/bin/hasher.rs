use std::{
    ffi::OsStr,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

use clap::Parser;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, types::chrono::Utc, Sqlite};
use tokio::try_join;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let db = init_db(&cli.sqlite).await?;

    let input = Path::new(&cli.input);

    let mut buf = Vec::with_capacity(150 * 1024 * 1014); // 150MiB

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
            if size as usize > buf.capacity() {
                eprintln!(
                    "File '{}' is too large, skipping, size: {}",
                    path.display(),
                    size
                );
                continue;
            }

            let start = Instant::now();

            entry.take(buf.capacity() as u64).read_to_end(&mut buf)?;
            let hash = seahash::hash(&buf);

            println!(
                "{}, size: {}, hash: 0x{hash:x}, elapsed: {}µs",
                path.display(),
                size,
                start.elapsed().as_micros()
            );

            if check_if_file_present(&db, hash).await? {
                println!("File already present in dataset, file='{}'", path.display());
                continue;
            }

            // TODO transcode to 16-bit mono wav, calculate  embedding, get duration,
            insert_file(&db, hash, cli.kind, &buf).await?;
        }
    } else {
        eprintln!("Input '{}' is not a tar-archive", input.display());
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

    sqlx::query(
        r"CREATE TABLE IF NOT EXISTS dataset_old (
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

    sqlx::query(
        r"CREATE TABLE IF NOT EXISTS dataset_new (
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

    let ((old_count,), (new_count,)): ((i64,), (i64,)) = try_join!(
        sqlx::query_as(r"SELECT COUNT(*) FROM dataset_old").fetch_one(&pool),
        sqlx::query_as(r"SELECT COUNT(*) FROM dataset_new").fetch_one(&pool),
    )?;
    if old_count > 0 {
        println!("dataset_old has {old_count} rows");
    }
    if new_count > 0 {
        println!("dataset_new has {new_count} rows");
    }

    Ok(pool)
}

async fn check_if_file_present(db: &SqlitePool, hash: u64) -> anyhow::Result<bool> {
    #[allow(clippy::cast_possible_wrap)]
    let hash = hash as i64;

    // First, check dataset_new and dataset_old, in case the previous run failed.
    // If the hash is present in these tables, then the file has been processed
    // but not yet persisted in the dataset table.
    // If not present, then check in the main dataset table.
    // If present in the main dataset table, copy it to dataset_old.

    let ((in_old,), (in_new,)): ((i64,), (i64,)) = try_join!(
        sqlx::query_as("SELECT COUNT(*) FROM dataset_old WHERE hash = ?")
            .bind(hash)
            .fetch_one(db),
        sqlx::query_as("SELECT COUNT(*) FROM dataset_new WHERE hash = ?")
            .bind(hash)
            .fetch_one(db),
    )?;

    if in_old > 0 || in_new > 0 {
        return Ok(true);
    }

    let res = sqlx::query(r"INSERT INTO dataset_old SELECT * FROM dataset WHERE hash = ?")
        .bind(hash)
        .execute(db)
        .await?;

    Ok(res.rows_affected() > 0)
}

async fn insert_file(
    db: &SqlitePool,
    hash: u64,
    kind: DatasetKind,
    content: &[u8],
) -> anyhow::Result<()> {
    #[allow(clippy::cast_possible_wrap)]
    let hash = hash as i64;

    let duration = 175;

    sqlx::query(
        r"INSERT INTO dataset_new (hash, content, kind, duration, embedding, added) VALUES(?,?,?,?,?,?)"
    ).bind(hash)
    .bind(content)
    .bind(kind.as_ref())
    .bind(duration)
    .bind(&[3,4,5][..])
    .bind(Utc::now())
    .execute(db).await?;

    Ok(())
}
