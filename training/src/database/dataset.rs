use sqlx::{sqlite::SqliteRow, Row};

use crate::entities::DatasetEntity;

use super::Database;

impl Database {
    pub(super) async fn init_dataset(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        println!("> table dataset");

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
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn has_in_dataset(&self, hash: i64) -> anyhow::Result<bool> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dataset WHERE hash = ?")
            .bind(hash)
            .fetch_one(&self.pool)
            .await?;

        Ok(count > 0)
    }

    pub async fn insert_into_dataset(&self, entity: DatasetEntity) -> anyhow::Result<()> {
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

    pub async fn get_any_from_dataset(&self) -> anyhow::Result<Option<DatasetEntity>> {
        let entity = sqlx::query("SELECT * FROM dataset ORDER BY RANDOM() LIMIT 1")
            .try_map(TryInto::try_into)
            .fetch_optional(&self.pool)
            .await?;
        Ok(entity)
    }

    pub async fn get_embedding(&self, hash: i64) -> anyhow::Result<Option<Vec<f32>>> {
        sqlx::query("SELECT embedding FROM dataset WHERE hash = ?")
            .bind(hash)
            .map(|row: SqliteRow| bytemuck::cast_slice(row.get("embedding")).to_vec())
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn get_file(&self, hash: i64) -> anyhow::Result<Option<(String, Vec<u8>)>> {
        sqlx::query("SELECT name, content FROM dataset WHERE hash = ?")
            .bind(hash)
            .map(|row: SqliteRow| (row.get("name"), row.get("content")))
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }
}
