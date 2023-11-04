use anyhow::bail;
use chrono::Utc;
use sqlx::{migrate::MigrateDatabase, sqlite::SqliteRow, Row, Sqlite, SqlitePool};

pub mod entities {

    use chrono::{DateTime, Duration, Utc};
    use sqlx::{sqlite::SqliteRow, Row};

    #[derive(Debug, Clone)]
    pub struct DatasetEntity {
        pub hash: i64, // PRIMARY KEY
        pub name: String,
        pub content: Vec<u8>,
        pub kind: super::model::DatasetKind,
        pub duration: Duration,
        pub embedding: Vec<f32>,
        pub added_at: DateTime<Utc>,
    }

    impl TryFrom<SqliteRow> for DatasetEntity {
        type Error = sqlx::Error;

        fn try_from(row: SqliteRow) -> Result<Self, Self::Error> {
            Ok(Self {
                hash: row.try_get("hash")?,
                name: row.try_get("name")?,
                content: row.try_get("content")?,
                kind: row.try_get("kind")?,
                duration: Duration::milliseconds(row.try_get("duration")?),
                embedding: bytemuck::cast_slice(row.try_get::<&[u8], _>("embedding")?).to_vec(),
                added_at: row.try_get("added")?,
            })
        }
    }

    #[derive(Debug, Clone)]
    pub struct ModelEntity {
        pub hash: i64, // PRIMARY KEY
        pub name: String,
        pub content: Vec<u8>,
        pub timestamp: DateTime<Utc>,
    }

    impl TryFrom<SqliteRow> for ModelEntity {
        type Error = sqlx::Error;

        fn try_from(row: SqliteRow) -> Result<Self, Self::Error> {
            Ok(Self {
                hash: row.try_get("hash")?,
                name: row.try_get("name")?,
                content: row.try_get("content")?,
                timestamp: row.try_get("timestamp")?,
            })
        }
    }
}

pub mod model {
    use std::fmt::Display;

    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, Copy, sqlx::Type)]
    pub enum DatasetKind {
        Advert,
        Music,
        Other,
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct ModelInfo {
        pub hash: i64,
        pub name: String,
        pub timestamp: DateTime<Utc>,
        pub size: i64,
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct ModelRun {
        pub id: i64,
        pub model_hash: i64,
        pub model_name: String,
        pub started: DateTime<Utc>,
        pub finished: Option<DateTime<Utc>>,
        pub files_count: i64,
    }

    impl Display for ModelInfo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!(
                "{:<15} {:>16x} {} {:>10}",
                self.name,
                self.hash,
                self.timestamp.to_rfc3339(),
                self.size
            ))
        }
    }

    impl Display for ModelRun {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!(
                "{:<2} {:<15} {:>16x} {} {} {:>10}",
                self.id,
                self.model_name,
                self.model_hash,
                self.started.to_rfc3339(),
                self.finished
                    .map_or_else(|| "N/A".to_string(), |f| f.to_rfc3339()),
                self.files_count
            ))
        }
    }
}

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

        sqlx::query(
            r"CREATE TABLE IF NOT EXISTS dataset (
                hash INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                content BLOB NOT NULL,
                kind TEXT NOT NULL,
                duration INTEGER NOT NULL,
                embedding BLOB NOT NULL,
                added TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r"CREATE TABLE IF NOT EXISTS models (
            hash INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            content BLOB NOT NULL,
            timestamp INTEGER NOT NULL
        )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r"CREATE TABLE IF NOT EXISTS model_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                model_hash INTEGER NOT NULL,
                started TEXT NOT NULL,
                finished TEXT,

                FOREIGN KEY(model_hash) REFERENCES models(hash)
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r"CREATE TABLE IF NOT EXISTS model_run_file_score (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER NOT NULL,
                file_hash INTEGER NOT NULL,
                score_advert REAL NOT NULL,
                score_music REAL NOT NULL,
                score_other REAL NOT NULL,

                FOREIGN KEY(run_id) REFERENCES model_runs(id),
                FOREIGN KEY(file_hash) REFERENCES dataset(hash)
            )",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn has_in_dataset(&self, hash: i64) -> anyhow::Result<bool> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dataset WHERE hash = ?")
            .bind(hash)
            .fetch_one(&self.pool)
            .await?;

        Ok(count > 0)
    }

    pub async fn insert_into_dataset(&self, entity: entities::DatasetEntity) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO dataset (hash, name, content, kind, duration, embedding, added) VALUES(?,?,?,?,?,?,?)",
        )
        .bind(entity.hash)
        .bind(&entity.name)
        .bind(&entity.content)
        .bind(entity.kind)
        .bind(entity.duration.num_milliseconds())
        .bind(bytemuck::cast_slice(entity.embedding.as_slice()))
        .bind(entity.added_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_any_from_dataset(&self) -> anyhow::Result<Option<entities::DatasetEntity>> {
        let entity = sqlx::query("SELECT * FROM dataset ORDER BY RANDOM() LIMIT 1")
            .try_map(TryInto::try_into)
            .fetch_optional(&self.pool)
            .await?;
        Ok(entity)
    }

    pub async fn insert_into_models(&self, name: &str, content: &[u8]) -> anyhow::Result<()> {
        #[allow(clippy::cast_possible_wrap)]
        sqlx::query(
            "INSERT OR IGNORE INTO models (hash, name, content, timestamp) VALUES(?,?,?,?)",
        )
        .bind(seahash::hash(content) as i64)
        .bind(name)
        .bind(content)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_models(&self) -> anyhow::Result<Vec<model::ModelInfo>> {
        sqlx::query_as(
            "SELECT hash, name, timestamp, length(content) as size FROM models ORDER BY name ASC, timestamp DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn start_model_run(&self, model_hash: i64) -> anyhow::Result<i64> {
        let res = sqlx::query("INSERT INTO model_runs (model_hash, started) VALUES(?,?)")
            .bind(model_hash)
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;

        Ok(res.last_insert_rowid())
    }

    pub async fn complete_model_run(&self, id: i64) -> anyhow::Result<()> {
        let res = sqlx::query("UPDATE model_runs SET finished = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() != 1 {
            bail!("Model run not found");
        }
        Ok(())
    }

    pub async fn list_model_runs(&self) -> anyhow::Result<Vec<model::ModelRun>> {
        sqlx::query_as(
            r"SELECT
                id,
                model_hash,
                (SELECT name FROM models WHERE hash = model_hash) as model_name,
                started,
                finished,
                (SELECT COUNT(*) FROM model_run_file_score WHERE run_id = id) as files_count
                FROM model_runs ORDER BY started DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_model(
        &self,
        name: Option<&str>,
        hash: Option<i64>,
    ) -> anyhow::Result<Option<model::ModelInfo>> {
        sqlx::query_as(
            r"SELECT hash, name, timestamp, length(content) as size
                FROM models
                WHERE hash = ? OR name = ?
                ORDER BY timestamp DESC
                LIMIT 1",
        )
        .bind(hash)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_model_content(&self, hash: i64) -> anyhow::Result<Option<Vec<u8>>> {
        sqlx::query("SELECT content FROM models WHERE hash = ?")
            .bind(hash)
            .map(|row: SqliteRow| row.get("content"))
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn select_file_indices_for_run(&self, run_id: i64) -> anyhow::Result<Vec<i64>> {
        sqlx::query(
            r"SELECT hash FROM dataset d
                LEFT JOIN model_run_file_score s
                ON d.hash = s.file_hash AND s.run_id = ? WHERE s.file_hash IS NULL",
        )
        .bind(run_id)
        .map(|row: SqliteRow| row.get("hash"))
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_model_run(&self, run_id: i64) -> anyhow::Result<Option<model::ModelRun>> {
        sqlx::query_as(
            r"SELECT
                id,
                model_hash,
                (SELECT name FROM models WHERE hash = model_hash) as model_name,
                started,
                finished,
                (SELECT COUNT(*) FROM model_run_file_score WHERE run_id = id) as files_count
                FROM model_runs
                WHERE id = ?",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }
}
