use std::str::FromStr;

use anyhow::Context;
use bytes::Bytes;
use model::{ContentKind, Tags};
use rocket::data::{ByteUnit, ToByteUnit};
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket_db_pools::Connection;
use serde::Serialize;

use crate::internal::prediction::Prediction;
use crate::internal::storage::Storage;
use crate::internal::{analyse, FingerprintMatch};

// struct AnalyseParams
// TODO - add pure post handler to be used from javascript or curl.
#[post("/segment/analyse", format = "plain", data = "<file>")]
pub async fn analyse_file(
    storage: Connection<Storage>,
    file: TempFile<'_>,
) -> Result<Json<AnalyseResponse>, status::Custom<String>> {
    let content = match &file {
        TempFile::Buffered { content } => Bytes::from(content.as_bytes().to_vec()),
        TempFile::File { .. } => file
            .path()
            .and_then(|p| std::fs::read(p).ok())
            .map(|v| v.into())
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
    log::info!("Analyse url: {url:#?}");
    let url = reqwest::Url::from_str(url)
        .map_err(|e| to_bad_request(format!("Invalid url: {url}. {e:#?}")))?;
    let content: Bytes = download(&url)
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
    content: &Bytes,
) -> anyhow::Result<AnalyseResponse> {
    analyse(storage, content).await.map(
        |(tags, content_kind_from_tags, fingerprints, predictions)| AnalyseResponse {
            tags,
            content_kind_from_tags,
            fingerprints,
            nn_predictions: predictions,
        },
    )
}

pub async fn download(url: &url::Url) -> anyhow::Result<Bytes> {
    let response = reqwest::get(url.clone()).await.context("Fetching {url}")?;

    if response.status() != reqwest::StatusCode::OK {
        anyhow::bail!(
            "Failed to fetch {url}: {} {}",
            response.status(),
            response.text().await?
        );
    }

    let content_length = ByteUnit::Byte(response.content_length().unwrap_or_default());
    let valid_length_range = 1.bytes()..2.mebibytes();
    if !valid_length_range.contains(&content_length) {
        anyhow::bail!(
            "Download failed, invalid content-length: {content_length}, must be in {}:{}, url={url}",
            valid_length_range.start, valid_length_range.end);
    }

    // Piggy back on auto conversion to anyhow::Error.
    let result = response.bytes().await?;
    Ok(result)
}
