#![allow(dead_code)]

use std::{collections::HashMap, str::FromStr};

use codec::AudioFrame;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

type Track = Vec<AudioFrame>;

pub struct AdsProvider {
    db_pool: SqlitePool,
    decoded_cache: HashMap<Uuid, Track>,
}

impl AdsProvider {
    pub async fn init() -> anyhow::Result<Self> {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")?;
        let db_pool = SqlitePool::connect_with(options).await?;

        init_db(&db_pool).await?;
        fill_db(&db_pool).await?;

        Ok(Self {
            db_pool,
            decoded_cache: HashMap::new(),
        })
    }

    pub async fn content(&self) -> anyhow::Result<Vec<(Uuid, String)>> {
        let rows = sqlx::query(r#"SELECT id, name FROM advertisements"#)
            .map(|row: SqliteRow| {
                (
                    uuid::Uuid::from_str(row.get("id")).unwrap(),
                    row.get("name"),
                )
            })
            .fetch_all(&self.db_pool)
            .await?;
        Ok(rows)
    }
}

async fn init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"CREATE TABLE "advertisements" (
            "id"	    TEXT NOT NULL UNIQUE COLLATE BINARY,
            "name"	    TEXT NOT NULL COLLATE NOCASE,
            "content"	BLOB NOT NULL COLLATE BINARY,
            PRIMARY KEY("id")
        )"#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn fill_db(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(r#"INSERT INTO "advertisements" (id, name, content) VALUES(?,?,?)"#)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind("Sample Advert")
        .bind(&(include_bytes!("../sample.aac"))[..])
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::{sqlite::SqliteRow, Row};
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn test_init() {
        let sut = AdsProvider::init().await.expect("Initialized provider");

        let rows = sqlx::query(r#"SELECT id, name, length(content) as size FROM advertisements"#)
            .map(|row: SqliteRow| {
                let id = Uuid::from_str(row.get("id")).expect("Valid UUIDv4");
                let name: &str = row.get("name");
                let size: i64 = row.get("size");
                format!("{id}: {name} / {size}")
            })
            .fetch_all(&sut.db_pool)
            .await
            .expect("Fetched rows");

        assert_eq!(1, rows.len());
    }

    #[tokio::test]
    async fn test_content() {
        let sut = AdsProvider::init().await.expect("Initialized provider");
        let content = sut.content().await.expect("Content list");

        dbg!(&content);

        assert_eq!(1, content.len());
        assert_eq!("Sample Advert", content[0].1);
    }
}
