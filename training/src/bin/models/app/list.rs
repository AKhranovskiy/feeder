use clap::{Args, Subcommand};

use training::Database;

#[derive(Args)]
pub struct ListArgs {
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

impl ListArgs {
    pub async fn run(&self, db: &Database) -> anyhow::Result<()> {
        match self.target {
            ListTarget::Models => {
                for model in db.list_models().await? {
                    println!("{model}");
                }
            }
            ListTarget::Runs => {
                for run in db.list_model_runs().await? {
                    println!("{run}");
                }
            }
        }

        Ok(())
    }
}
