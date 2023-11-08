use std::{
    ffi::OsStr,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::Utc;
use clap::{Args, Subcommand};
use kdam::BarExt;
use tap::TapFallible;
use tokio::{sync::Mutex, task::JoinSet};

use training::{database::Database, model::DatasetKind};

const WORKERS: usize = 10;
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MiB
const QUEUE_LIMIT: usize = 10;

#[derive(Args)]
pub struct AddArgs {
    #[command(subcommand)]
    source: AddSource,
}

#[derive(Subcommand)]
pub enum AddSource {
    Data {
        /// Data source. Supported sources: tar-archive, directory.
        #[arg(short, long)]
        input: PathBuf,

        /// Dataset kind
        #[arg(short, long)]
        kind: DatasetKind,
    },
    Model {
        /// Path to model export dir
        path: PathBuf,
    },
}

impl AddArgs {
    pub async fn run(&self, db: &Database) -> anyhow::Result<()> {
        match &self.source {
            AddSource::Data { input, kind } => add_files(db, input, *kind).await,
            AddSource::Model { path } => add_model(db, path).await,
        }
    }
}

async fn add_model(db: &Database, path: &Path) -> anyhow::Result<()> {
    let model_file: PathBuf = path.join("saved_model.pb");
    if !model_file.exists() {
        anyhow::bail!("{} not found", model_file.display());
    }

    let model_name = path
        .file_name()
        .and_then(|f| f.to_str())
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("Invalid model path '{}'", path.display()))?;

    println!("model_name: {model_name}");

    println!("Archiving model...");
    let content = {
        let mut ar = tar::Builder::new(Vec::new());
        ar.append_dir_all(".", path)?;
        ar.into_inner()?
    };

    println!("Archive size: {}", content.len());

    println!("Inserting into database...");

    db.insert_into_models(&model_name, &content).await?;

    Ok(())
}

async fn add_files(db: &Database, input: &Path, kind: DatasetKind) -> anyhow::Result<()> {
    let pb = progress_bar()?;

    let (task_sender, task_receiver) = flume::bounded::<Task>(QUEUE_LIMIT);

    let mut workers = spawn_workers(&task_receiver, db, &pb);

    if input.is_file() && input.extension() == Some(OsStr::new("tar")) {
        process_tar(input, kind, &task_sender)?;
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

fn progress_bar() -> anyhow::Result<Arc<Mutex<kdam::Bar>>> {
    kdam::BarBuilder::default()
        .unit("files")
        .desc("Processed")
        .force_refresh(true)
        .build()
        .map_err(|s| anyhow::anyhow!(s))
        .map(Mutex::new)
        .map(Arc::new)
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

#[derive(Debug, Clone)]
struct Task {
    name: String,
    hash: i64,
    content: Vec<u8>,
    kind: training::model::DatasetKind,
    embedding: Vec<f32>,
    duration: chrono::Duration,
}

impl From<Task> for training::entities::DatasetEntity {
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

fn process_tar(
    input: &Path,
    kind: DatasetKind,
    task_sender: &flume::Sender<Task>,
) -> anyhow::Result<()> {
    println!("Processing tar-archive '{}'", input.display());

    let mut archive = tar::Archive::new(std::fs::File::open(input)?);

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
            kind,
            embedding: vec![],
            duration: chrono::Duration::zero(),
        })?;
    }

    Ok(())
}

async fn worker(
    task_receiver: flume::Receiver<Task>,
    db: Database,
    pb: Arc<Mutex<kdam::Bar>>,
) -> anyhow::Result<()> {
    let yamnet = classifier::Yamnet::load("./models").tap_err(|err| {
        eprintln!("Failed to load yamnet: {err}");
    })?;

    while let Ok(task) = task_receiver.recv_async().await {
        let name = task.name.clone();

        let in_db = db.has_in_dataset(task.hash).await.tap_err(|err| {
            eprintln!("Failed to check if file already in dataset, file='{name}', {err:#}");
        })?;

        if in_db {
            pb.lock().await.update(1).tap_err(|err| {
                eprintln!("Failed to update progress bar for file={name}, err={err:#}");
            })?;

            continue;
        }

        let task = process(&yamnet, task).tap_err(|err| {
            eprintln!("Failed to process file='{name}', {err}");
        })?;

        db.insert_into_dataset(task.into()).await.tap_err(|err| {
            eprintln!("Failed to store file={name}, {err:#}");
        })?;

        pb.lock().await.update(1).tap_err(|err| {
            eprintln!("Failed to update progress bar for file={name}, err={err:#}",);
        })?;
    }

    Ok(())
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
