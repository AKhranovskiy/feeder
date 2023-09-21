use axum::{extract::State, response::Html, routing::get, Router};
use minijinja::render;
use serde::Serialize;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/playbacks", get(playbacks))
        .with_state(state)
}

const PLAYBACKS_TEMPLATE: &str = include_str!("../../templates/playbacks.html");

async fn playbacks(State(state): State<AppState>) -> Result<Html<String>, ()> {
    let records = state.ads_provider.playbacks().await.map_err(|_| ())?;
    let records = records
        .into_iter()
        .map(PlaybackRecord::from)
        .collect::<Vec<_>>();
    log::debug!("Playback records: {records:?}");

    let r = render!(PLAYBACKS_TEMPLATE, records => records);
    Ok(Html(r))
}

#[derive(Debug, Serialize)]
struct PlaybackRecord {
    id: String,
    name: String,
    started: String,
    finished: String,
}

impl From<crate::ads_management::PlaybackRecord> for PlaybackRecord {
    fn from(record: crate::ads_management::PlaybackRecord) -> Self {
        Self {
            id: record.id.to_string(),
            name: record.name,
            started: record.started.format("%Y-%m-%d %H:%M:%S").to_string(),
            finished: record.finished.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}
