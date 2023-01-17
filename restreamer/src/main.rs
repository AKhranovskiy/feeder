use analyzer::BufferedAnalyzer;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, get_service};
use axum::{Router, Server};
use tower_http::services::ServeDir;

mod terminate;
use self::terminate::Terminator;

mod play_params;
mod route_play;

#[tokio::main]
async fn main() {
    let serve_dir = get_service(ServeDir::new("assets")).handle_error(handle_error);
    let terminator = Terminator::new();

    // TODO warm up NN
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

async fn handle_error(_err: std::io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}
