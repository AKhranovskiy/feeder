use anyhow::{anyhow, Context};
use model::{ContentKind, Segment, SegmentInsertResponse, SegmentMatchResponse};
use uuid::Uuid;

const MIN_CONFIDENCE: f32 = 0.2f32;
const EMYSOUND_API: &str = "http://localhost:3340/api/v1.1/";

pub async fn find_matches(segment: &Segment) -> anyhow::Result<Option<Vec<SegmentMatchResponse>>> {
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
        emysound::query(EMYSOUND_API, "media.file", &segment.content, MIN_CONFIDENCE)
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

pub async fn add_fingerprints(
    segment: &Segment,
    kind: ContentKind,
) -> anyhow::Result<SegmentInsertResponse> {
    let id = Uuid::new_v4();

    let artist = segment
        .tags
        .track_artist()
        .map(ToString::to_string)
        .unwrap_or_default();

    let title = segment
        .tags
        .track_title()
        .map(ToString::to_string)
        .unwrap_or_default();

    emysound::insert(
        EMYSOUND_API.parse()?,
        id,
        artist.clone(),
        title.clone(),
        segment.url.clone(),
        &segment.content,
    )
    .await
    .context("EmySound::insert")
    .map(|()| SegmentInsertResponse {
        id,
        artist,
        title,
        kind,
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

pub async fn delete_segment(id: &str) -> anyhow::Result<()> {
    let id = uuid::Uuid::parse_str(id)?;
    emysound::delete(EMYSOUND_API, id).await
}
