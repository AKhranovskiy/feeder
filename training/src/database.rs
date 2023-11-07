use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

mod dataset;
mod file_score;
mod model_runs;
mod models;

#[derive(Debug, Clone)]
pub struct Database {
    pool: sqlx::SqlitePool,
}

impl Database {
    pub async fn init(path: &str) -> anyhow::Result<Self> {
        println!("Initializing Sqlite DB at '{path}'");

        let url = format!("sqlite://{path}");

        if !Sqlite::database_exists(&url).await? {
            println!("Creating database '{path}'");
            Sqlite::create_database(&url).await?;
        }

        let pool = SqlitePool::connect(&url).await?;

        Self::init_dataset(&pool).await?;
        Self::init_models(&pool).await?;
        Self::init_model_runs(&pool).await?;
        Self::init_file_score(&pool).await?;

        Ok(Self { pool })
    }
}
