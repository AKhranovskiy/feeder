use anyhow::{Context, Result};
use clap::Parser;

mod app;
use app::{App, Args};

#[tokio::main]
async fn main() -> Result<()> {
    init_logger()?;

    App::run(Args::parse()).await.context("Running the app")?;

    Ok(())
}

fn init_logger() -> Result<()> {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;
    Ok(())
}
