use std::{hash::Hash, str::FromStr, sync::Arc};

use chrono::{DateTime, Utc};
use codec::{AudioFrame, CodecParams};
use sqlx::{sqlite::SqliteRow, FromRow, Row, SqlitePool};
use uuid::Uuid;

use super::{AdCache, AdId};

type Track = Vec<AudioFrame>;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ContentItem {
    pub id: AdId,
    pub name: String,
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

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct PlaybackRecord {
    pub client_id: Uuid,
    pub track_id: AdId,
    pub name: String,
    pub started: DateTime<Utc>,
    pub finished: DateTime<Utc>,
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

    pub async fn content(&self) -> anyhow::Result<Vec<ContentItem>> {
        let items = sqlx::query_as::<_, ContentItem>(r#"SELECT id, name FROM advertisements"#)
            .fetch_all(&self.db_pool)
            .await?;

        Ok(items)
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
            .bind(id)
            .map(|row: SqliteRow| row.get("content"))
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Content not found"))?;

        self.cache.insert(id, &content).await?;
        let track = self.cache.get(id, target_params).await?;
        Ok(track)
    }

    #[allow(clippy::unused_async)]
    pub async fn report_started(&self, client_id: Uuid, id: AdId) -> anyhow::Result<()> {
        log::info!("Client {}: start playing item {}", client_id, id.as_ref());

        Ok(())
    }

    pub async fn report_finished(
        &self,
        client_id: Uuid,
        track_id: AdId,
        started: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        log::info!(
            "Client {}: finished playing item {}",
            client_id,
            track_id.as_ref()
        );
        sqlx::query(
            r#"INSERT INTO "playbacks" (client_id, track_id, started, finished) VALUES(?,?,?,?)"#,
        )
        .bind(client_id)
        .bind(track_id)
        .bind(started)
        .bind(Utc::now())
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    pub async fn playbacks(&self) -> anyhow::Result<Vec<PlaybackRecord>> {
        let records = sqlx::query_as::<_, PlaybackRecord>(
            r#"
                SELECT p.client_id, p.track_id, a.name, p.started, p.finished FROM "playbacks" p
                LEFT JOIN advertisements a ON a.id = p.track_id
                ORDER BY p.finished DESC, p.started DESC;
            "#,
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(records)
    }

    #[allow(dead_code)]
    pub async fn playbacks_by_id(&self, id: AdId) -> anyhow::Result<Vec<PlaybackRecord>> {
        let records = sqlx::query_as::<_, PlaybackRecord>(
            r#"
                SELECT p.client_id, p.track_id, a.name, p.started, p.finished FROM "playbacks" p
                LEFT JOIN advertisements a ON a.id = p.id
                WHERE p.id = ?
                ORDER BY p.finished DESC, p.started DESC;
            "#,
        )
        .bind(id)
        .fetch_all(&self.db_pool)
        .await?;

        Ok(records)
    }
}

async fn init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"CREATE TABLE "advertisements" (
            "id"	    TEXT NOT NULL UNIQUE,
            "name"	    TEXT NOT NULL,
            "content"	BLOB NOT NULL,
            PRIMARY KEY("id")
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE "playbacks" (
            "client_id"     TEXT NOT NULL,
            "track_id"	    TEXT NOT NULL,
            "started"	    TEXT NOT NULL,
            "finished"	    TEXT NOT NULL
        )"#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn fill_db(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(r#"INSERT INTO "advertisements" (id, name, content) VALUES(?,?,?)"#)
        .bind(AdId::new())
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
            .bind(id)
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
    use std::time::Duration;

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
        assert_eq!("Sample Advert", content[0].name);
    }

    #[tokio::test]
    async fn test_playbacks() {
        let sut = AdsProvider::init().await.expect("Initialized provider");
        let content = sut.content().await.expect("Content items");
        let id = content[0].id;
        let client_id = Uuid::new_v4();

        let started = Utc::now();
        sut.report_started(client_id, id).await.expect("Started");
        tokio::time::sleep(Duration::from_millis(200)).await;
        sut.report_finished(client_id, id, started)
            .await
            .expect("Finished");

        let playbacks = sut.playbacks().await.expect("Playback records");

        dbg!(&playbacks);

        assert_eq!(1, playbacks.len());
    }
}
