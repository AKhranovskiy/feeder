use clap::Parser;

use training::Database;

mod add;
mod commands;
mod list;
mod resume;
mod run;

use self::commands::Commands;

pub async fn run() -> anyhow::Result<()> {
    let app = App::parse();

    let db = Database::init(&app.db).await?;
    app.command.run(&db).await?;

    Ok(())
}

#[derive(Parser)]
pub struct App {
    /// Sqlite DB path
    #[arg(long)]
    db: String,

    #[command(subcommand)]
    command: Commands,
}
