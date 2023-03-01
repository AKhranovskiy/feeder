#![allow(clippy::module_name_repetitions)]

use analyzer::BufferedAnalyzer;
use axum::routing::{get, get_service};
use axum::{Router, Server};
use tower_http::services::ServeDir;

mod terminate;
use self::terminate::Terminator;

mod mixer;
mod play_params;
mod route_play;
mod stream_saver;

#[tokio::main]
async fn main() {
    let serve_dir = get_service(ServeDir::new("assets"));
    let terminator = Terminator::new();

    BufferedAnalyzer::warmup();

    let app = Router::new()
        .nest_service("/", serve_dir.clone())
        .route("/play", get(route_play::serve))
        .with_state(terminator.clone());

    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .with_graceful_shutdown(terminator.signal())
        .await
        .unwrap();
}
