use std::{hash::Hash, str::FromStr, sync::Arc};

use codec::{AudioFrame, CodecParams};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use super::{AdCache, AdId};

type Track = Vec<AudioFrame>;

#[derive(Debug, Clone, sqlx::FromRow)]
struct ContentItem {
    id: uuid::fmt::Hyphenated,
    name: String,
}

impl PartialEq for ContentItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ContentItem {}

impl Hash for ContentItem {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct AdsProvider {
    db_pool: SqlitePool,
    cache: AdCache,
}

impl AdsProvider {
    pub async fn init() -> anyhow::Result<Self> {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")?;
        let db_pool = SqlitePool::connect_with(options).await?;

        init_db(&db_pool).await?;
        fill_db(&db_pool).await?;

        Ok(Self {
            db_pool,
            cache: AdCache::new(),
        })
    }

    pub async fn content(&self) -> anyhow::Result<Vec<(AdId, String)>> {
        let items = sqlx::query_as::<_, ContentItem>(r#"SELECT id , name FROM advertisements"#)
            .fetch_all(&self.db_pool)
            .await?;

        Ok(items
            .into_iter()
            .map(|item| ((*item.id.as_uuid()).into(), item.name))
            .collect())
    }

    pub async fn get(
        &self,
        id: AdId,
        target_params: CodecParams,
    ) -> anyhow::Result<Option<Arc<Track>>> {
        log::info!("Get advertisement, id={}", id.as_ref().to_string());

        let item = self.cache.get(id, target_params).await?;
        if item.is_some() {
            return Ok(item);
        }

        let content: Vec<u8> = sqlx::query(r#"SELECT content FROM advertisements WHERE id=?"#)
            .bind(id.as_ref().to_string())
            .map(|row: SqliteRow| row.get("content"))
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Content not found"))?;

        // decode
        self.cache.insert(id, &content).await?;
        self.cache.get(id, target_params).await
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
        .bind(Uuid::new_v4().to_string())
        .bind("Sample Advert")
        .bind(&(include_bytes!("../../sample.aac"))[..])
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
impl AdsProvider {
    pub async fn testing(track: Track) -> Self {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
        let db_pool = SqlitePool::connect_with(options).await.unwrap();
        init_db(&db_pool).await.unwrap();

        let id = AdId::new();
        sqlx::query(r#"INSERT INTO "advertisements" (id, name, content) VALUES(?,?,?)"#)
            .bind(id.as_ref().to_string())
            .bind("Test")
            .bind(&[0, 1, 2][..])
            .execute(&db_pool)
            .await
            .unwrap();

        Self {
            db_pool,
            cache: AdCache::build_testing(id, track),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init() {
        AdsProvider::init().await.expect("Initialized provider");
    }

    #[tokio::test]
    async fn test_content() {
        let sut = AdsProvider::init().await.expect("Initialized provider");
        let content = sut.content().await.expect("Content items");

        assert_eq!(1, content.len());
        assert_eq!("Sample Advert", content[0].1);
    }
}
