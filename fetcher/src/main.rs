use anyhow::{Context, Result};
use clap::Parser;

mod app;
pub mod utils;

use app::{App, Args};

#[tokio::main]
async fn main() -> Result<()> {
    init_logger()?;

    App::run(Args::parse()).await.context("Running the app")?;

    Ok(())
}

fn init_logger() -> Result<()> {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::ConfigBuilder::new()
            .set_time_format_rfc3339()
            .add_filter_allow("fetcher::app".to_owned())
            .build(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;
    Ok(())
}
