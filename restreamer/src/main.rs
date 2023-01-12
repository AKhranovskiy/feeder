use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use analyzer::{BufferedAnalyzer, ContentKind, LabelSmoother};
use async_stream::stream;
use axum::body::StreamBody;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, get_service};
use axum::{Router, Server};
use codec::{Decoder, Encoder};
use futures::Stream;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use tokio::signal;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct Shutdown {
    is_terminated: Arc<AtomicBool>,
}

impl Shutdown {
    fn new() -> Self {
        Self {
            is_terminated: Arc::new(AtomicBool::new(false)),
        }
    }

    fn shutdown(&self) {
        self.is_terminated.store(true, Ordering::Relaxed);
    }

    fn is_terminated(&self) -> bool {
        self.is_terminated.load(Ordering::Relaxed)
    }
}

#[tokio::main]
async fn main() {
    let serve_dir = get_service(ServeDir::new("assets")).handle_error(handle_error);

    let shutdown = Shutdown::new();

    let app = Router::new()
        .nest_service("/", serve_dir.clone())
        .route("/play", get(play))
        .with_state(shutdown.clone());

    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal(shutdown.clone()))
        .await
        .unwrap();
}

async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

async fn play(
    Query(params): Query<PlayParams>,
    State(shutdown): State<Shutdown>,
) -> StreamBody<impl Stream<Item = anyhow::Result<Vec<u8>>>> {
    stream! {
        let (mut reader, writer) = os_pipe::pipe()?;

        let sh = shutdown.clone();


        std::thread::spawn(move || {
        let action = params.action.unwrap_or(PlayAction::Passthrough);

            let input = unstreamer::Unstreamer::open(params.url)?;

            let decoder = Decoder::try_from(input)?;

            let mut encoder = Encoder::opus(decoder.codec_params(), writer)?;

            let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(7));

            for frame in decoder {
                let frame = frame?;

                let kind = analyzer.push(frame.clone())?;
                if action == PlayAction::Silence && kind == ContentKind::Advertisement {
                    encoder.push(codec::silence_frame(&frame))?;
                } else {
                    encoder.push(frame)?;
                }
                std::io::stdout().write_all(&kind.name().as_bytes()[..1])?;
                std::io::stdout().flush()?;

                if sh.is_terminated() {
                    break;
                }
            }

            encoder.flush()?;

            std::io::stdout().write_all("\n\nTerminating analyzer".as_bytes())?;
            std::io::stdout().flush()?;

            anyhow::Ok(())
        });

        let mut buf = [0u8;4096];
        while !shutdown.is_terminated() {
            let read = reader.read(&mut buf)?;
            yield Ok(buf[0..read].to_vec())
        }
    }
    .into()
}

#[derive(Debug, Deserialize)]
struct PlayParams {
    url: url::Url,
    #[serde(deserialize_with = "deserialize_lang")]
    action: Option<PlayAction>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum PlayAction {
    Passthrough,
    Silence,
    Lang(String),
}

fn deserialize_lang<'de, D>(de: D) -> Result<Option<PlayAction>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(value) = Option::<String>::deserialize(de)? else { return Ok(Some(PlayAction::Passthrough)) };

    match value.to_lowercase().as_str() {
        "passthrough" => Ok(Some(PlayAction::Passthrough)),
        "silence" => Ok(Some(PlayAction::Silence)),
        lang if lang.len() == 2 => Ok(Some(PlayAction::Lang(lang.into()))),
        value => Err(Error::custom(format!(
            "expected Passthrough, Silence or Lang, received {value}"
        ))),
    }
}

async fn shutdown_signal(shutdown: Shutdown) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => { shutdown.shutdown(); },
        _ = terminate => { shutdown.shutdown(); },
    }

    println!("signal received, starting graceful shutdown");
}
