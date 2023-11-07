
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
