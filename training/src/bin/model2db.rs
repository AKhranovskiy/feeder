use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;

use training::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let model_file: PathBuf = cli.model.join("saved_model.pb");
    if !model_file.exists() {
        bail!("{} not found", model_file.display());
    }

    let model_name = cli
        .model
        .file_name()
        .and_then(|f| f.to_str())
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("Invalid model path '{:?}'", cli.model))?;

    println!("model_name: {model_name}");

    println!("Archiving model...");

    let mut ar = tar::Builder::new(Vec::new());
    ar.append_dir_all(".", cli.model)?;
    let content = ar.into_inner()?;

    println!("Archive size: {}", content.len());

    let db = Database::init(&cli.db).await?;

    println!("Inserting into database...");
    db.insert_into_models(&model_name, &content).await?;

    Ok(())
}

#[derive(Parser)]
struct Cli {
    /// Sqlite file
    #[arg(long)]
    db: String,

    /// Model dir
    #[arg(long)]
    model: PathBuf,
}
