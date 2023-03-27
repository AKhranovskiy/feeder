use std::convert::AsRef;
use std::path::PathBuf;
use std::sync::Arc;

use analyzer::BufferedAnalyzer;
use axum::routing::{get, get_service};
use axum::{Router, Server};
use rand::seq::SliceRandom;
use tower_http::services::ServeDir;

mod adbuffet;

mod terminate;
use terminate::Terminator;

use self::adbuffet::AdBuffet;

mod routes;

#[derive(Clone)]
struct GlobalState {
    pub ad_buffet: Arc<AdBuffet>,
    pub terminator: Terminator,
}

#[tokio::main]
async fn main() {
    let serve_dir = get_service(ServeDir::new("assets"));

    BufferedAnalyzer::warmup();

    let state = GlobalState {
        ad_buffet: Arc::new(load_ads()),
        terminator: Terminator::new(),
    };

    eprintln!("AdBuffet loaded {} ads", state.ad_buffet.size());

    let app = Router::new()
        .nest_service("/", serve_dir.clone())
        .nest(
            "/vast",
            routes::vast::routes("localhost:3000", state.clone()),
        )
        .route("/play", get(routes::play::serve))
        .with_state(state.clone());

    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .with_graceful_shutdown(state.terminator.signal())
        .await
        .unwrap();
}

fn load_ads() -> AdBuffet {
    let mut list = include_str!("../assets/files/list.txt")
        .lines()
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    list.shuffle(&mut rand::thread_rng());

    let refs = list.iter().map(AsRef::as_ref).collect::<Vec<_>>();

    AdBuffet::try_from(refs.as_slice()).expect("Should load all ads")
}
