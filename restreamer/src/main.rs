use analyzer::BufferedAnalyzer;
use axum::routing::{get, get_service};
use axum::{Router, Server};
use tower_http::services::ServeDir;

mod adbuffet;

mod terminate;
use terminate::Terminator;

mod routes;

#[tokio::main]
async fn main() {
    let serve_dir = get_service(ServeDir::new("assets"));
    let terminator = Terminator::new();

    BufferedAnalyzer::warmup();

    let app = Router::new()
        .nest_service("/", serve_dir.clone())
        .nest(
            "/vast",
            routes::vast::routes("localhost:3000", terminator.clone()),
        )
        .route("/play", get(routes::play::serve))
        .with_state(terminator.clone());

    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .with_graceful_shutdown(terminator.signal())
        .await
        .unwrap();
}
