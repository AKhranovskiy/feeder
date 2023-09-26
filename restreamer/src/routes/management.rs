use axum::{extract::State, response::Html, routing::get, Router};
use minijinja::render;
use serde::Serialize;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/playbacks", get(playbacks))
        .with_state(state)
}

// const PLAYBACKS_TEMPLATE: &str = include_str!("../../templates/playbacks.html");

fn live_playback_template() -> String {
    std::fs::read_to_string("restreamer/templates/playbacks.html").unwrap()
}

async fn playbacks(State(state): State<AppState>) -> Result<Html<String>, ()> {
    let records = state.ads_provider.playbacks().await.map_err(|_| ())?;
    let records = records
        .into_iter()
        .map(PlaybackRecord::from)
        .collect::<Vec<_>>();
    log::debug!("Playback records: {records:?}");

    let r = render!(&live_playback_template(), records => records);
    Ok(Html(r))
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
