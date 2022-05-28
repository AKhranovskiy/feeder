mod matcher;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use url::Url;
use uuid::Uuid;

use self::matcher::best_results;

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub id: Uuid,
    pub coverage: u8,
    pub artist: String,
    pub title: String,
}

impl TryFrom<&emysound::QueryResult> for QueryResult {
    type Error = anyhow::Error;

    fn try_from(value: &emysound::QueryResult) -> Result<Self, Self::Error> {
        let id = Uuid::try_parse(&value.track.id).context("Parsing uuid")?;
        let coverage = value
            .audio
            .as_ref()
            .and_then(|audio| audio.coverage.query_coverage)
            .map(|coverage| (255f32 * coverage).trunc() as u8)
            .ok_or_else(|| anyhow!("Failed to get coverage"))?;
        let artist = value.track.artist.clone().unwrap_or_default();
        let title = value.track.title.clone().unwrap_or_default();

        Ok(Self {
            id,
            coverage,
            artist,
            title,
        })
    }
}

const MIN_CONFIDENCE: f32 = 0.2f32;
const EMYSOUND_API: &str = "http://localhost:3340/api/v1.1/";

pub async fn query(filename: &str, content: &Bytes) -> anyhow::Result<Vec<QueryResult>> {
    emysound::query(
        Url::parse(EMYSOUND_API)?,
        filename.to_string(),
        content.clone(),
        MIN_CONFIDENCE,
    )
    .await
    .context("EmySound::query")?
    .iter()
    .map(|result| result.try_into())
    .inspect(|result| log::debug!("{result:?}"))
    .collect::<anyhow::Result<Vec<_>>>()
    .map(best_results)
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub id: Uuid,
    pub artist: String,
    pub title: String,
}

impl TrackInfo {
    pub fn new(id: Uuid, artist: String, title: String) -> Self {
        Self { id, artist, title }
    }
}

pub async fn insert(info: TrackInfo, filename: &str, content: &Bytes) -> anyhow::Result<()> {
    emysound::insert(
        EMYSOUND_API.parse()?,
        info.id,
        info.artist,
        info.title,
        filename.to_string(),
        content.clone(),
    )
    .await
    .context("EmySound::insert")
}
