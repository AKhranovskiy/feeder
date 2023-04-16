#![allow(clippy::module_name_repetitions)]

use std::net::SocketAddr;

use analyzer::BufferedAnalyzer;
use axum::routing::{get, get_service};
use axum::{Router, Server};
use tower_http::services::ServeDir;

mod terminate;
use terminate::Terminator;

mod routes;
mod stream_saver;

#[tokio::main]
async fn main() {
    let serve_dir = get_service(ServeDir::new("assets"));
    let terminator = Terminator::new();

    BufferedAnalyzer::warmup();

    let app = Router::new()
        .nest_service("/", serve_dir.clone())
        .route("/play", get(routes::play::serve))
        .with_state(terminator.clone());

    Server::bind(&get_addr())
        .serve(app.into_make_service())
        .with_graceful_shutdown(terminator.signal())
        .await
        .unwrap();
}

fn get_addr() -> SocketAddr {
    let port = std::env::args()
        .nth(1)
        .map(|p| p.parse::<u16>())
        .transpose()
        .expect("Valid port")
        .unwrap_or(15190);
    assert!(port >= 3000);
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), port);
    println!("Listening on {addr}");
    addr
}
