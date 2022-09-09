// Rocket's FromForm generates code with a warning. It could be already fixed in the latest version of Rocket.
#![allow(clippy::unnecessary_lazy_evaluations)]

use std::str::FromStr;

use model::{ContentKind, Tags};
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::{get, post};
use rocket_db_pools::Connection;
use serde::Serialize;
use url::Url;

use crate::internal::prediction::Prediction;
use crate::internal::storage::Storage;
use crate::internal::{analyse, download, FingerprintMatch};

// struct AnalyseParams
// TODO - add pure post handler to be used from javascript or curl.
#[post("/segment/analyse", format = "plain", data = "<file>")]
pub async fn analyse_file(
    storage: Connection<Storage>,
    file: TempFile<'_>,
) -> Result<Json<AnalyseResponse>, status::Custom<String>> {
    let content = match &file {
        TempFile::Buffered { content } => content.as_bytes().to_vec(),
        TempFile::File { .. } => file
            .path()
            .and_then(|p| std::fs::read(p).ok())
            .ok_or_else(|| anyhow::anyhow!("Failed to read file"))
            .map_err(|e| to_bad_request(e.to_string()))?,
    };
    analyse_impl(storage, &content)
        .await
        .map(Json)
        .map_err(|e| {
            log::error!("{e:#?}");
            status::Custom(Status::InternalServerError, e.to_string())
        })
}

#[derive(Debug, Serialize)]
pub struct AnalyseResponse {
    pub tags: Tags,
    pub content_kind_from_tags: ContentKind,
    pub fingerprints: Vec<FingerprintMatch>,
    pub nn_predictions: Vec<Prediction>,
}

#[get("/segment/analyse?<url>")]
pub async fn analyse_url(
    storage: Connection<Storage>,
    url: &str,
) -> Result<Json<AnalyseResponse>, status::Custom<String>> {
    let url =
        Url::from_str(url).map_err(|e| to_bad_request(format!("Invalid url: {url}. {e:#?}")))?;

    let (_, content) = download(url)
        .await
        .map_err(|e| to_bad_request(e.to_string()))?;

    analyse_impl(storage, &content)
        .await
        .map(Json)
        .map_err(|e| {
            log::error!("{e:#?}");
            status::Custom(Status::InternalServerError, e.to_string())
        })
}

fn to_bad_request(msg: String) -> status::Custom<String> {
    status::Custom(Status::BadRequest, msg)
}
pub async fn analyse_impl<'i>(
    storage: Connection<Storage>,
    content: &[u8],
) -> anyhow::Result<AnalyseResponse> {
    analyse(&storage, content, "").await.map(
        |(tags, content_kind_from_tags, fingerprints, predictions)| AnalyseResponse {
            tags,
            content_kind_from_tags,
            fingerprints,
            nn_predictions: predictions,
        },
    )
}
