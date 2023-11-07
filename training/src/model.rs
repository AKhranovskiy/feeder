
use std::fmt::Display;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, sqlx::Type)]
pub enum DatasetKind {
    Advert,
    Music,
    Other,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModelInfo {
    pub hash: i64,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub size: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModelRun {
    pub id: i64,
    pub model_hash: i64,
    pub model_name: String,
    pub started: DateTime<Utc>,
    pub finished: Option<DateTime<Utc>>,
    pub files_count: i64,
}

impl Display for ModelInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:<15} {:>16x} {} {:>10}",
            self.name,
            self.hash,
            self.timestamp.to_rfc3339(),
            self.size
        ))
    }
}

impl Display for ModelRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:<2} {:<15} {:>16x} {} {} {:>10}",
            self.id,
            self.model_name,
            self.model_hash,
            self.started.to_rfc3339(),
            self.finished
                .map_or_else(|| "N/A".to_string(), |f| f.to_rfc3339()),
            self.files_count
        ))
    }
}
