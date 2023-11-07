use sqlx::{sqlite::SqliteRow, Row};

use super::Database;

impl Database {
    pub(super) async fn init_file_score(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        println!("> table file_score");

        sqlx::query(
            r"CREATE TABLE IF NOT EXISTS file_score (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER NOT NULL,
                file_hash INTEGER NOT NULL,
                predictions BLOB NOT NULL,
                shape BLOB NOT NULL,
                confidence_advert REAL NOT NULL,
                confidence_music REAL NOT NULL,
                confidence_other REAL NOT NULL,

                FOREIGN KEY(run_id) REFERENCES model_runs(id),
                FOREIGN KEY(file_hash) REFERENCES dataset(hash)
            )",
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn select_file_indices_for_run(&self, run_id: i64) -> anyhow::Result<Vec<i64>> {
        sqlx::query(
            r"SELECT hash FROM dataset d
                LEFT JOIN file_score s
                ON d.hash = s.file_hash AND s.run_id = ? WHERE s.file_hash IS NULL",
        )
        .bind(run_id)
        .map(|row: SqliteRow| row.get("hash"))
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn insert_file_score(
        &self,
        run_id: i64,
        file_hash: i64,
        predictions: &[f32],
        shape: &[usize],
        confidence: [f32; 3],
    ) -> anyhow::Result<()> {
        sqlx::query(
            r"INSERT INTO file_score
                (run_id, file_hash, predictions, shape, confidence_advert, confidence_music, confidence_other)
                VALUES(?,?,?,?,?,?,?)"
            )
            .bind(run_id)
            .bind(file_hash)
            .bind(bytemuck::cast_slice(predictions))
            .bind(bytemuck::cast_slice(shape))
            .bind(confidence[0])
            .bind(confidence[1])
            .bind(confidence[2])
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
