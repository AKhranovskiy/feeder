use clap::{Parser, Subcommand};
use reqwest::Url;

#[derive(Debug, Parser)]
struct Args {
    /// The Emy Sound endpoint.
    #[clap(short, long, default_value = "http://localhost:3340/api/v1.1/")]
    endpoint: Url,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Insert,
    Query,
    Delete {
        /// An id of the file to delete.
        #[clap(value_parser)]
        id: uuid::Uuid,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Insert => todo!(),
        Command::Query => todo!(),
        Command::Delete { id } => emysound::delete(args.endpoint.as_str(), id).await,
    }
}
