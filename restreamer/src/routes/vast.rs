use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use rand::Rng;

use crate::terminate::Terminator;

pub fn routes(terminator: Terminator) -> Router<Terminator> {
    Router::new().route("/", get(root)).with_state(terminator)
}

const VAST_FILES: &[&str] = &[include_str!("../../vast/one.xml")];

#[allow(clippy::unused_async)]
async fn root(State(_terminator): State<Terminator>) -> impl IntoResponse {
    let id = rand::thread_rng().gen_range(0..VAST_FILES.len());
    let xml = VAST_FILES[id].replace("{{SERVER}}", "localhost:3000");

    Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "application/xml")
        .body(xml)
        .unwrap()
}
