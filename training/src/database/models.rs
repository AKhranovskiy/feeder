use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row};

use crate::model::ModelInfo;

use super::Database;

impl Database {
    pub(super) async fn init_models(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        println!("> table models");

        sqlx::query(
            r"CREATE TABLE IF NOT EXISTS models (
            hash INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            content BLOB NOT NULL,
            timestamp INTEGER NOT NULL
        )",
        )
        .execute(pool)
        .await?;

        Ok(())
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

    pub async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        sqlx::query_as(
            "SELECT hash, name, timestamp, length(content) as size FROM models ORDER BY name ASC, timestamp DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_model(
        &self,
        name: Option<&str>,
        hash: Option<i64>,
    ) -> anyhow::Result<Option<ModelInfo>> {
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
}
