use std::{
    ffi::OsStr,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use chrono::{Duration, Utc};
use clap::Parser;
use kdam::BarExt;
use tokio::{sync::Mutex, task::JoinSet};
use tools::Database;

const WORKERS: usize = 10;
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MiB
const QUEUE_LIMIT: usize = 10;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let input = Path::new(&cli.input);

    let db = Database::init(&cli.sqlite).await?;
    let pb = progress_bar()?;

    let (task_sender, task_receiver) = flume::bounded::<Task>(QUEUE_LIMIT);

    let mut workers = spawn_workers(&task_receiver, &db, &pb);

    if input.is_file() && input.extension() == Some(OsStr::new("tar")) {
        process_tar(input, cli.kind, &task_sender)?;
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

impl From<DatasetKind> for tools::model::DatasetKind {
    fn from(kind: DatasetKind) -> Self {
        match kind {
            DatasetKind::Advert => Self::Advert,
            DatasetKind::Music => Self::Music,
            DatasetKind::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone)]
struct Task {
    name: String,
    hash: i64,
    content: Vec<u8>,
    kind: tools::model::DatasetKind,
    embedding: Vec<f32>,
    duration: chrono::Duration,
}

impl From<Task> for tools::entities::DatasetEntity {
    fn from(task: Task) -> Self {
        Self {
            hash: task.hash,
            name: task.name,
            content: task.content,
            kind: task.kind,
            duration: task.duration,
            embedding: task.embedding,
            added_at: Utc::now(),
        }
    }
}

fn spawn_workers(
    task_receiver: &flume::Receiver<Task>,
    db: &Database,
    pb: &Arc<Mutex<kdam::Bar>>,
) -> JoinSet<anyhow::Result<()>> {
    let mut workers = JoinSet::new();

    for _ in 0..WORKERS {
        let receiver = task_receiver.clone();
        let db = db.clone();
        let pb = pb.clone();

        workers.spawn(async move { worker(receiver, db, pb).await });
    }
    workers
}

async fn worker(
    task_receiver: flume::Receiver<Task>,
    db: Database,
    pb: Arc<Mutex<kdam::Bar>>,
) -> anyhow::Result<()> {
    let yamnet = match classifier::Yamnet::load("./models") {
        Ok(yamnet) => yamnet,
        Err(err) => {
            panic!("Failed to load yamnet: {err}");
        }
    };

    while let Ok(task) = task_receiver.recv_async().await {
        let name = task.name.clone();

        if db.has(task.hash).await? {
            if let Err(error) = pb.lock().await.update(1) {
                eprintln!("Failed to update progress bar for task={name}, err={error}");
            }

            let _ = pb
                .lock()
                .await
                .write(format!("File already present in dataset, file='{name}'"));

            continue;
        }

        match process(&yamnet, task) {
            Ok(task) => match db.insert(task.into()).await {
                Ok(()) => {
                    if let Err(err) = pb.lock().await.update(1) {
                        eprintln!("Failed to update progress bar for task={name}, err={err}",);
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
}

fn process(yamnet: &classifier::Yamnet, task: Task) -> anyhow::Result<Task> {
    let wav = codec::resample_16k_mono_s16_stream(task.content.as_slice())?;

    let duration =
        chrono::Duration::milliseconds((wav.len() as f32 / 16_000.0 * 1_000.0).trunc() as i64);

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

fn progress_bar() -> anyhow::Result<Arc<Mutex<kdam::Bar>>> {
    kdam::BarBuilder::default()
        .unit("files")
        .desc("Processed")
        .force_refresh(true)
        .build()
        .map_err(|s| anyhow!(s))
        .map(Mutex::new)
        .map(Arc::new)
}

fn process_tar(
    input: &Path,
    kind: DatasetKind,
    task_sender: &flume::Sender<Task>,
) -> anyhow::Result<()> {
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

        #[allow(clippy::cast_possible_wrap)]
        let hash = seahash::hash(&content) as i64;

        task_sender.send(Task {
            name: path.display().to_string(),
            hash,
            content,
            kind: kind.into(),
            embedding: vec![],
            duration: Duration::zero(),
        })?;
    }

    Ok(())
}
