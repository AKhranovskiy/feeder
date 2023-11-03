use clap::Parser;
use training::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let db = Database::init(&cli.db).await?;

    if let Some(entity) = db.get_any().await? {
        println!("Found entity: {} {}", entity.hash, entity.duration);
    }

    Ok(())
}

#[derive(clap::Parser)]
struct Cli {
    /// Sqlite file
    #[arg(long)]
    db: String,

    /// Listen port
    #[arg(short, long, default_value = "8080")]
    port: u16,
}
