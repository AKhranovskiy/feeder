use std::{net::SocketAddr, sync::Arc};

use ads_management::AdsProvider;
use axum::{routing::get_service, Router, Server};
use clap::Parser;
use log::LevelFilter;
use stderrlog::Timestamp;
use tower_http::services::ServeDir;

use args::Args;
use codec::configure_ffmpeg_log;

mod accept_header;
mod ads_management;
mod args;
mod rate;
mod routes;
mod stream_saver;
mod terminate;

use terminate::Terminator;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    configure_logger(&args);

    let serve_dir = get_service(ServeDir::new("restreamer/assets"));
    let terminator = Terminator::new();
    let ads_provider = Arc::new(AdsProvider::init().await.expect("AdsProvider"));

    let app = Router::new().nest_service("/", serve_dir.clone()).nest(
        "/play",
        routes::play::router(terminator.clone(), ads_provider, args.clone()),
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
        .module("analyzer::analyzer")
        .module("analyzer::smooth")
        .module("codec")
        .module("codec::dsp::cross_fader")
        .module("restreamer")
        .module("restreamer::routes::play")
        .module("restreamer::stream_saver")
        .module("restreamer::terminate")
        .quiet(args.quiet);

    if args.gcp {
        log.verbosity(LevelFilter::Info);
    } else {
        log.verbosity(LevelFilter::Debug);
        log.timestamp(Timestamp::Millisecond);
    }

    log.init().unwrap();

    configure_ffmpeg_log();
}
