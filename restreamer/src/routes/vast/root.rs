use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};

use super::state::VastState;

#[allow(clippy::unused_async)]
pub async fn serve(State(state): State<VastState>) -> impl IntoResponse {
    Response::builder()
        .header(CONTENT_TYPE, "application/xml")
        .body(state.collection.get_random())
        .unwrap()
}
