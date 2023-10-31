use std::{hash::Hash, str::FromStr, sync::Arc};

use anyhow::ensure;
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
    pub duration: u32,
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

#[derive(Debug, Clone, FromRow)]
pub struct PlaybackRecord {
    pub client_id: Uuid,
    pub track_id: AdId,
    pub name: String,
    pub started: DateTime<Utc>,
    pub finished: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct TrackRecord {
    pub id: AdId,
    pub name: String,
    pub added: DateTime<Utc>,
    pub duration: u32,
    pub played: u32,
}

impl AdsProvider {
    pub async fn init() -> anyhow::Result<Self> {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")?;
        let db_pool = SqlitePool::connect_with(options).await?;

        init_db(&db_pool).await?;

        Ok(Self {
            db_pool,
            cache: AdCache::new(),
        })
    }

    pub async fn content(&self) -> anyhow::Result<Vec<ContentItem>> {
        let items = sqlx::query_as::<_, ContentItem>("SELECT id, name, duration FROM tracks")
            .fetch_all(&self.db_pool)
            .await?;

        Ok(items)
    }

    pub async fn get(
        &self,
        id: AdId,
        target_params: CodecParams,
    ) -> anyhow::Result<Option<Arc<Track>>> {
        log::info!("Get track, id={}", id.as_ref().to_string());

        let item = self.cache.get(id, target_params).await?;
        if item.is_some() {
            return Ok(item);
        }

        let content: Vec<u8> = sqlx::query("SELECT content FROM tracks WHERE id=?")
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
            "INSERT INTO playbacks (client_id, track_id, started, finished) VALUES(?,?,?,?)",
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
            r"
                SELECT p.client_id, p.track_id, t.name, p.started, p.finished FROM playbacks p
                LEFT JOIN tracks t ON t.id = p.track_id
                ORDER BY p.finished DESC, p.started DESC;
            ",
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(records)
    }

    pub async fn playbacks_by_id(&self, id: AdId) -> anyhow::Result<Vec<PlaybackRecord>> {
        let records = sqlx::query_as::<_, PlaybackRecord>(
            r"
                SELECT p.client_id, p.track_id, t.name, p.started, p.finished FROM playbacks p
                LEFT JOIN tracks t ON t.id = p.track_id
                WHERE p.track_id = ?
                ORDER BY p.finished DESC, p.started DESC;
            ",
        )
        .bind(id)
        .fetch_all(&self.db_pool)
        .await?;

        Ok(records)
    }

    pub async fn tracks(&self) -> anyhow::Result<Vec<TrackRecord>> {
        let records = sqlx::query_as::<_, TrackRecord>(
            r"
                SELECT t.id, t.name, t.duration, t.added,
                    (SELECT count(*) FROM playbacks p WHERE p.track_id = t.id) as played
                FROM tracks t
                ORDER BY t.added DESC;
            ",
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(records)
    }

    pub async fn add_track(&self, name: &str, content: &[u8]) -> anyhow::Result<AdId> {
        let codec_params = codec::track_codec_params(content)?;
        ensure!(codec_params.is_valid(), "Invalid codec params");

        let duration = codec::track_duration(content)?.as_secs();
        let id = AdId::new();

        sqlx::query(
            "INSERT INTO tracks (id, name, content, added, duration) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(content)
        .bind(Utc::now())
        .bind(duration as u32)
        .execute(&self.db_pool)
        .await?;

        Ok(id)
    }
}

async fn init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"CREATE TABLE tracks (
            "id"	    TEXT NOT NULL UNIQUE,
            "name"	    TEXT NOT NULL,
            "content"	BLOB NOT NULL,
            "added"     TEXT NOT NULL,
            "duration"  INTEGER NOT NULL,
            PRIMARY KEY("id")
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE playbacks (
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

#[cfg(test)]
impl AdsProvider {
    pub async fn testing(track: Track) -> Self {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
        let db_pool = SqlitePool::connect_with(options).await.unwrap();
        init_db(&db_pool).await.unwrap();

        let id = AdId::new();
        sqlx::query("INSERT INTO tracks (id, name, content, added, duration) VALUES(?,?,?,?,?)")
            .bind(id)
            .bind("Test track")
            .bind(&[0, 1, 2][..])
            .bind(Utc::now())
            .bind(3)
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
        let sut = AdsProvider::testing(vec![]).await;
        let content = sut.content().await.expect("Content items");

        assert_eq!(1, content.len());
        assert_eq!("Test track", content[0].name);
    }

    #[tokio::test]
    async fn test_playbacks() {
        let sut = AdsProvider::testing(vec![]).await;
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

        assert_eq!(1, playbacks.len());
    }

    #[tokio::test]
    async fn test_playback_by_id() {
        let sut = AdsProvider::testing(vec![]).await;
        let content = sut.content().await.expect("Content items");
        let id = content[0].id;
        let client_id = Uuid::new_v4();

        let started = Utc::now();
        sut.report_started(client_id, id).await.expect("Started");
        tokio::time::sleep(Duration::from_millis(200)).await;
        sut.report_finished(client_id, id, started)
            .await
            .expect("Finished");

        let playbacks = sut.playbacks_by_id(id).await.expect("Playback records");

        assert_eq!(1, playbacks.len());
    }

    #[tokio::test]
    async fn test_tracks() {
        let sut = AdsProvider::testing(vec![]).await;
        let content = sut.content().await.expect("Content items");
        let id = content[0].id;
        let client_id = Uuid::new_v4();

        let started = Utc::now();
        sut.report_started(client_id, id).await.expect("Started");
        tokio::time::sleep(Duration::from_millis(200)).await;
        sut.report_finished(client_id, id, started)
            .await
            .expect("Finished");

        let tracks = sut.tracks().await.expect("Track records");

        dbg!(&tracks);

        assert_eq!(1, tracks.len());
        assert_eq!(1, tracks[0].played);
    }
}
