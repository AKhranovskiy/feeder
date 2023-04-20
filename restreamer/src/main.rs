#![allow(clippy::module_name_repetitions)]

use std::net::SocketAddr;

use analyzer::BufferedAnalyzer;
use args::Args;
use axum::routing::get_service;
use axum::{Router, Server};
use clap::Parser;
use log::LevelFilter;
use stderrlog::Timestamp;
use tower_http::services::ServeDir;

mod args;
mod terminate;
use terminate::Terminator;

mod routes;
mod stream_saver;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    configure_logger(&args);

    let serve_dir = get_service(ServeDir::new("assets"));
    let terminator = Terminator::new();

    BufferedAnalyzer::warmup();

    let app = Router::new().nest_service("/", serve_dir.clone()).nest(
        "/play",
        routes::play::router(terminator.clone(), args.clone()),
    );

    Server::bind(&get_addr(&args))
        .serve(app.into_make_service())
        .with_graceful_shutdown(terminator.signal())
        .await
        .unwrap();
}

fn get_addr(args: &Args) -> SocketAddr {
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), args.port);
    log::info!("Listening on {addr}");
    addr
}

fn configure_logger(args: &Args) {
    let mut log = stderrlog::new();
    log.show_module_names(false)
        .show_level(false)
        .module("restreamer")
        .module("restreamer::stream_saver")
        .module("analyzer::smooth")
        .module("codec::dsp::cross_fader")
        .module("analyzer::analyzer")
        .quiet(args.quiet);

    if args.gcp {
        log.verbosity(LevelFilter::Info);
    } else {
        log.verbosity(LevelFilter::Debug);
        log.timestamp(Timestamp::Second);
    }

    log.init().unwrap();
}
