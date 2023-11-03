use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

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
}

pub mod model {
    #[derive(Debug, Clone, Copy, sqlx::Type)]
    pub enum DatasetKind {
        Advert,
        Music,
        Other,
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

        Ok(Self { pool })
    }

    pub async fn has(&self, hash: i64) -> anyhow::Result<bool> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dataset WHERE hash = ?")
            .bind(hash)
            .fetch_one(&self.pool)
            .await?;

        Ok(count > 0)
    }

    pub async fn insert(&self, entity: entities::DatasetEntity) -> anyhow::Result<()> {
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

    pub async fn get_any(&self) -> anyhow::Result<Option<entities::DatasetEntity>> {
        let entity = sqlx::query("SELECT * FROM dataset ORDER BY RANDOM() LIMIT 1")
            .try_map(TryInto::try_into)
            .fetch_optional(&self.pool)
            .await?;
        Ok(entity)
    }
}
