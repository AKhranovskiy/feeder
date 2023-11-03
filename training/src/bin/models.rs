use std::path::PathBuf;

use anyhow::bail;
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use training::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let db = Database::init(&cli.db).await?;

    match cli.command {
        Commands::List(ListArgs { target }) => match target {
            ListTarget::Models => {
                let models = db.list_models().await?;
                for model in models {
                    println!(
                        "{:<15} {:>16x} {} {:>10}",
                        model.name,
                        model.hash,
                        model.timestamp.to_rfc3339(),
                        model.size
                    );
                }
            }
            ListTarget::Runs => {
                let runs = db.list_model_runs().await?;
                for run in runs {
                    println!(
                        "{:<15} {:>16x} {} {} {:>10}",
                        run.model_name,
                        run.model_hash,
                        run.started.to_rfc3339(),
                        run.finished
                            .map_or_else(|| "N/A".to_string(), |f| f.to_rfc3339()),
                        run.files_count
                    );
                }
            }
        },
        Commands::Run { name, .. } => {
            anyhow::ensure!(name.is_some());

            #[allow(clippy::cast_possible_wrap)]
            let hash = name
                .as_ref()
                .and_then(|s| u64::from_str_radix(s, 16).ok())
                .map(|h| h as i64);

            let Some(model) = db.find_model(name.as_deref(), hash.as_ref()).await? else {
                bail!("Model '{}' not found", name.unwrap());
            };

            println!(
                "{:<15} {:>16x} {} {:>10}",
                model.name,
                model.hash,
                model.timestamp.to_rfc3339(),
                model.size
            );

            let Some(content) = db.get_model_content(model.hash).await? else {
                bail!("Model content not found");
            };

            tar::Archive::new(content.as_slice()).unpack(format!(
                "/run/user/1000/adbanda-model-{}-{}",
                model.hash,
                Utc::now().timestamp()
            ))?;

            if model.name.ends_with("_amt") {
                println!("Running AMT...");

                // TODO Load AMT model,
                // Iterate over embeddings in batches, write results to DB
            }
        }
        Commands::Add { path } => {
            let model_file: PathBuf = path.join("saved_model.pb");
            if !model_file.exists() {
                bail!("{} not found", model_file.display());
            }

            let model_name = path
                .file_name()
                .and_then(|f| f.to_str())
                .map(ToString::to_string)
                .ok_or_else(|| anyhow::anyhow!("Invalid model path '{}'", path.display()))?;

            println!("model_name: {model_name}");

            println!("Archiving model...");

            let mut ar = tar::Builder::new(Vec::new());
            ar.append_dir_all(".", path)?;
            let content = ar.into_inner()?;

            println!("Archive size: {}", content.len());

            println!("Inserting into database...");

            db.insert_into_models(&model_name, &content).await?;
        }
    }

    Ok(())
}

#[derive(Parser)]
struct Cli {
    /// Sqlite DB path
    #[arg(long)]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available models.
    List(ListArgs),
    /// Run a model.
    #[group(required = true, multiple = false)]
    Run {
        /// By name (latest timestamp).
        name: Option<String>,
        /// By hash
        hash: Option<String>,
    },
    /// Add a model to DB.
    Add {
        /// Path to model export dir
        path: PathBuf,
    },
}

#[derive(Args)]
struct ListArgs {
    #[command(subcommand)]
    target: ListTarget,
}

#[derive(Subcommand)]
enum ListTarget {
    /// List models.
    Models,
    /// List runs.
    Runs,
}
