use anyhow::{anyhow, Context};
use model::{ContentKind, Segment, SegmentInsertResponse, SegmentMatchResponse};
use url::Url;
use uuid::Uuid;

const MIN_CONFIDENCE: f32 = 0.2f32;
const EMYSOUND_API: &str = "http://localhost:3340/api/v1.1/";

pub async fn find_matches(segment: &Segment) -> anyhow::Result<Option<Vec<SegmentMatchResponse>>> {
    let endpoint = Url::parse(EMYSOUND_API)?;
    let filename = segment.url.path().to_string();
    let content = segment.content.clone();

    let to_results = |results: Vec<emysound::QueryResult>| {
        results
            .iter()
            .map(|r| r.try_into())
            .collect::<anyhow::Result<Vec<QueryResult>>>()
    };
    let to_responses = |results: Vec<QueryResult>| {
        results
            .iter()
            .map(|r| r.into())
            .collect::<Vec<SegmentMatchResponse>>()
    };

    let matches: Vec<SegmentMatchResponse> =
        emysound::query(endpoint, filename, content, MIN_CONFIDENCE)
            .await
            .context("EmySound::query")
            .and_then(to_results)
            .map(best_results)
            .map(to_responses)?;

    Ok(if matches.is_empty() {
        None
    } else {
        Some(matches)
    })
}

pub async fn insert_segment(segment: &Segment) -> anyhow::Result<SegmentInsertResponse> {
    let id = Uuid::new_v4();
    let artist = segment.artist();
    let title = segment.title();
    // TODO content kind

    let filename = segment.url.path().to_string();
    let content = segment.content.clone();

    emysound::insert(
        EMYSOUND_API.parse()?,
        id,
        artist.clone(),
        title.clone(),
        filename,
        content,
    )
    .await
    .context("EmySound::insert")
    .map(|()| SegmentInsertResponse {
        id,
        artist,
        title,
        kind: ContentKind::Unknown,
    })
}

#[derive(Debug, Clone)]
struct QueryResult {
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

impl From<&QueryResult> for SegmentMatchResponse {
    fn from(result: &QueryResult) -> Self {
        SegmentMatchResponse {
            id: result.id,
            score: result.coverage,
            artist: result.artist.clone(),
            title: result.title.clone(),
            kind: ContentKind::Unknown,
        }
    }
}

fn best_results(results: Vec<QueryResult>) -> Vec<QueryResult> {
    results
        .iter()
        .filter(|r| {
            if r.coverage >= 190 {
                true
            } else {
                results
                    .iter()
                    .find(|r2| {
                        r2.id != r.id
                            && (r2.artist == r.artist || r2.title == r.title)
                            && r.coverage.checked_add(r2.coverage).unwrap_or_default() > 230
                    })
                    .map(|v| {
                        log::debug!("Result match: {r:?} - {v:?}");
                        v
                    })
                    .is_some()
            }
        })
        .cloned()
        .collect()
}
