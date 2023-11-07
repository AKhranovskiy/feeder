use clap::Subcommand;

use training::Database;

#[derive(Subcommand)]
pub enum Commands {
    /// Add a model to DB.
    Add(super::add::AddArgs),
    /// List available models.
    List(super::list::ListArgs),
    /// Run a model.
    #[group(required = true, multiple = false)]
    Run(super::run::RunArgs),
    /// Resume an incomplete model run.
    Resume(super::resume::ResumeArgs),
    /// Get object from DB
    Get(super::get::GetArgs),
}

impl Commands {
    pub async fn run(&self, db: &Database) -> anyhow::Result<()> {
        match self {
            Self::Add(args) => args.run(db).await,
            Self::Get(args) => args.run(db).await,
            Self::List(args) => args.run(db).await,
            Self::Resume(args) => args.run(db).await,
            Self::Run(args) => args.run(db).await,
        }
    }
}
