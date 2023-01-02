use std::io;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, get_service};
use axum::{Router, Server};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let serve_dir = get_service(ServeDir::new("assets")).handle_error(handle_error);

    let app = Router::new()
        .nest_service("/", serve_dir.clone())
        .route("/play", get(play));

    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

async fn play(Query(params): Query<PlayParams>) -> String {
    format!(
        "{:?} {}",
        params.action.unwrap_or(PlayAction::Passthrough),
        params.url.as_str()
    )
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PlayParams {
    url: url::Url,
    #[serde(deserialize_with = "deserialize_lang")]
    action: Option<PlayAction>,
}

#[derive(Debug, Deserialize)]
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
