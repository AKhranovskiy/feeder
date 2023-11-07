use chrono::Utc;

use crate::model::ModelRun;

use super::Database;

impl Database {
    pub(super) async fn init_model_runs(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        println!("> table model_runs");

        sqlx::query(
            r"CREATE TABLE IF NOT EXISTS model_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                model_hash INTEGER NOT NULL,
                started TEXT NOT NULL,
                finished TEXT,

                FOREIGN KEY(model_hash) REFERENCES models(hash)
            )",
        )
        .execute(pool)
        .await?;

        Ok(())
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
            anyhow::bail!("Model run not found");
        }
        Ok(())
    }

    pub async fn list_model_runs(&self) -> anyhow::Result<Vec<ModelRun>> {
        sqlx::query_as(
            r"SELECT
                id,
                model_hash,
                (SELECT name FROM models WHERE hash = model_hash) as model_name,
                started,
                finished,
                (SELECT COUNT(*) FROM file_score WHERE run_id = id) as files_count
                FROM model_runs ORDER BY started DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_model_run(&self, run_id: i64) -> anyhow::Result<Option<ModelRun>> {
        sqlx::query_as(
            r"SELECT
                id,
                model_hash,
                (SELECT name FROM models WHERE hash = model_hash) as model_name,
                started,
                finished,
                (SELECT COUNT(*) FROM file_score WHERE run_id = id) as files_count
                FROM model_runs
                WHERE id = ?",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }
}
