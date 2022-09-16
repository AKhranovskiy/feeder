use anyhow::{anyhow, ensure, Context, Result};
use reqwest::header::{HeaderMap, ACCEPT};
use reqwest::multipart::{Form, Part};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use uuid::Uuid;

pub async fn check_connection(endpoint: &str) -> Result<()> {
    let url = Url::parse(endpoint)?.join("Streams")?;
    Client::new()
        .get(url)
        .basic_auth("ADMIN", Some(""))
        .send()
        .await?
        .error_for_status()
        .map(|_| ())
        .map_err(|e| e.into())
}

pub async fn insert(
    endpoint: Url,
    id: Uuid,
    artist: String,
    title: String,
    filename: String,
    content: &[u8],
) -> Result<()> {
    log::trace!(
        target: "emysound::insert",
        "endpoint={endpoint}, id={id}, artist={artist}, title={title}, filename={filename}, content:len={}",
        content.len()
    );

    let headers = {
        let mut h = HeaderMap::new();
        h.insert(ACCEPT, "application/json".parse().unwrap());
        h
    };

    let form = Form::new()
        .text("Id", id.to_string())
        .text("Artist", artist)
        .text("Title", title)
        .text("MediaType", "Audio")
        .part(
            "file",
            Part::stream(content.to_vec())
                .file_name(filename)
                .mime_str("application/octet-stream")
                .context("Attaching content")?,
        );

    let url = endpoint.join("Tracks")?;
    let res = Client::new()
        .post(url)
        .basic_auth("ADMIN", Some(""))
        .headers(headers)
        .multipart(form)
        .send()
        .await
        .context("Sending query to EmySound")?;

    let status = res.status();

    if status == StatusCode::OK {
        Ok(())
    } else {
        let text = res.text().await?;
        Err(anyhow!("Failed to insert track {status} {text}"))
    }
}

pub async fn query(
    endpoint: &str,
    filename: &str,
    content: &[u8],
    min_confidence: f32,
) -> Result<Vec<QueryResult>> {
    log::trace!(
        target: "emysound::query",
        "endpoint={endpoint}, filename={filename}, content:len={}, min_confidence={min_confidence:.02}",
        content.len()
    );
    ensure!(
        (0f32..=1f32).contains(&min_confidence),
        "Min confidence must be between 0 and 1"
    );

    let headers = {
        let mut h = HeaderMap::new();
        h.insert(ACCEPT, "application/json".parse().unwrap());
        h
    };

    let form = Form::new().part(
        "file",
        Part::stream(content.to_vec())
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .context("Attaching content")?,
    );

    let url = Url::parse(endpoint)?.join("Query")?;

    let res = Client::new()
        .post(url)
        .basic_auth("ADMIN", Some(""))
        .headers(headers)
        .query(&[
            ("mediaType", "Audio"),
            ("minCoverage", &min_confidence.to_string()),
        ])
        .multipart(form)
        .send()
        .await
        .context("Sending query to EmySound")?;

    let status = res.status();

    if status == StatusCode::OK {
        res.json().await.context("Decode response body failed")
    } else {
        let text = res.text().await?;
        Err(anyhow!("Failed to query track {status} {text}"))
    }
}

pub async fn delete(endpoint: &str, id: uuid::Uuid) -> anyhow::Result<()> {
    log::trace!(target: "emysound::delete", "endpoint={endpoint}, id={id}");

    let url = Url::parse(endpoint)?
        .join("Tracks/")?
        .join(&id.to_string())?;

    println!("DELETE {url}");

    let res = Client::new()
        .delete(url)
        .basic_auth("ADMIN", Some(""))
        .send()
        .await
        .context("Sending query to EmySound")?;

    match res.status() {
        StatusCode::OK => Ok(()),
        StatusCode::NOT_FOUND => Err(anyhow!("File not found, id={id}")),
        StatusCode::INTERNAL_SERVER_ERROR => {
            Err(anyhow!("Internal server error: {}", res.text().await?))
        }
        _ => Err(anyhow!("Unexpected status code: {}", res.status())),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    /// Unique ID for a query match. You can use this ID to search for query matches in Emy /api/v1/matches endpoint.
    pub id: String,
    /// Object containing track information.
    pub track: TrackInfo,
    /// Query match object.
    pub audio: Option<AudioMatch>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    /// Track unique identifier.
    pub id: String,
    /// Track title.
    pub title: Option<String>,
    /// Track artist.
    pub artist: Option<String>,
    /// Audio track length, measured in seconds.
    #[serde(rename = "audioTrackLength")]
    pub length: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMatch {
    /// Query match unique identifier.
    #[serde(rename = "queryMatchId")]
    pub id: String,
    /// Object containing information about query match coverage.
    pub coverage: AudioCoverage,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioCoverage {
    /// Query match starting position in seconds.
    pub query_match_starts_at: f32,
    /// Track match starting position in seconds.
    pub track_match_starts_at: f32,
    /// Gets relative query coverage, calculated by dividing QueryCoverageLength by QueryLength.
    pub query_coverage: Option<f32>,
    /// Gets relative track coverage, calculated by dividing TrackCoverageLength by TrackLength.
    pub track_coverage: Option<f32>,
    /// Query coverage length in seconds. Shows how many seconds from the query have been covered in the track.
    pub query_coverage_length: f32,
    /// Track coverage length in seconds. Shows how many seconds form the track have been covered in the query.
    pub track_coverage_length: f32,
    /// Discrete query coverage length in seconds. It is calculated by summing QueryCoverageLength with QueryGaps.
    pub query_discrete_coverage_length: f32,
    /// Discrete track coverage length in seconds. It is calculated by summing TrackCoverageLength with TrackGaps.
    pub track_discrete_coverage_length: f32,
    /// Query length in seconds.
    pub query_length: f32,
    /// Track length in seconds.
    pub track_length: f32,
    /// List of identified gaps in the query.
    pub query_gaps: Vec<Gap>,
    /// List of identified gaps in the track.
    pub track_gaps: Vec<Gap>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gap {
    /// Starting position of the gap in seconds.
    pub start: f32,
    /// Ending position of the gap in seconds.
    pub end: f32,
    /// Value indicating whether the gap is on the very beginning or very end.
    pub is_on_edge: bool,
    /// Gets length in seconds calculated by the difference: End - Start.
    pub length_in_seconds: f32,
}
