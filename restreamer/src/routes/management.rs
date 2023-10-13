use std::path::PathBuf;

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use minijinja::render;
use serde::Serialize;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/playbacks", get(playbacks))
        .route("/playbacks/:track_id", get(playbacks_by_id))
        .route("/tracks", get(tracks).post(upload))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(25 * 1024 * 1024 /* 25mb */))
        .with_state(state)
}

// const PLAYBACKS_TEMPLATE: &str = include_str!("../../templates/playbacks.html");

fn live_playback_template() -> String {
    std::fs::read_to_string("restreamer/templates/playbacks.html").unwrap()
}

fn live_tracks_template() -> String {
    std::fs::read_to_string("restreamer/templates/tracks.html").unwrap()
}

async fn playbacks(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let records = state.ads_provider.playbacks().await?;

    let records = records
        .into_iter()
        .map(PlaybackRecord::from)
        .collect::<Vec<_>>();
    log::debug!("Playback records: {records:?}");

    let r = render!(&live_playback_template(), records => records);
    Ok(Html(r))
}

async fn playbacks_by_id(
    Path(track_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let records = state
        .ads_provider
        .playbacks_by_id(track_id.parse()?)
        .await?;

    let records = records
        .into_iter()
        .map(PlaybackRecord::from)
        .collect::<Vec<_>>();
    log::debug!("Playback records: {records:?}");

    let r = render!(&live_playback_template(), track => track_id, records => records);
    Ok(Html(r))
}

async fn tracks(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let records = state.ads_provider.tracks().await?;
    let records = records
        .into_iter()
        .map(TrackRecord::from)
        .collect::<Vec<_>>();
    log::debug!("Track records: {records:?}");

    let r = render!(&live_tracks_template(), records => records);
    Ok(Html(r))
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Html<String>, AppError> {
    if let Some(field) = multipart.next_field().await? {
        let file_name = field
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("No file name"))?
            .to_string();

        let track_name = PathBuf::from(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?
            .to_owned();

        let data = field.bytes().await?;

        log::info!("Uploaded `{track_name}` of size {} bytes", data.len());

        state.ads_provider.add_track(&track_name, &data).await?;
    } else {
        log::info!("No file uploaded");
        Err(anyhow::anyhow!("No file uploaded"))?;
    }

    tracks(State(state)).await
}

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Debug, Serialize)]
struct PlaybackRecord {
    client_id: String,
    track_id: String,
    name: String,
    started: String,
    finished: String,
}

impl From<crate::ads_management::PlaybackRecord> for PlaybackRecord {
    fn from(record: crate::ads_management::PlaybackRecord) -> Self {
        Self {
            client_id: record.client_id.to_string(),
            track_id: record.track_id.to_string(),
            name: record.name,
            started: record.started.format("%Y-%m-%d %H:%M:%S").to_string(),
            finished: record.finished.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct TrackRecord {
    track_id: String,
    name: String,
    duration: String,
    added: String,
    played: String,
}

impl From<crate::ads_management::TrackRecord> for TrackRecord {
    fn from(record: crate::ads_management::TrackRecord) -> Self {
        Self {
            track_id: record.id.to_string(),
            name: record.name,
            duration: format!("{} s", record.duration),
            added: record.added.format("%Y-%m-%d %H:%M:%S").to_string(),
            played: record.played.to_string(),
        }
    }
}
