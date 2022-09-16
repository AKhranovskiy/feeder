use std::str::FromStr;

use anyhow::{anyhow, bail, Context};
use rocket::put;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::Responder;
use rocket_db_pools::Connection;
use serde::Deserialize;
use url::Url;

use crate::{
    internal::{download, storage::Storage},
    storage::{streams::StreamDocumentMatch, StorageScheme},
};

#[derive(Debug, Deserialize)]
pub struct FetchRequest<'r> {
    name: &'r str,
    url: &'r str,
}

impl<'r> FetchRequest<'r> {
    fn validate(&self) -> anyhow::Result<(&str, Url)> {
        if self.name.is_empty() {
            bail!("Name may not be empty.")
        }
        Url::from_str(self.url)
            .map(|url| (self.name, url))
            .map_err(|e| anyhow!("URL is invalid: {e}"))
    }
}

#[derive(Debug, Responder)]
pub enum FetchError {
    #[response(status = 400)]
    BadRequest(String),
    #[response(status = 409)]
    Conflict(Json<Vec<String>>),
    #[response(status = 500)]
    ServerError(String),
}

/// Adds new stream for fetching.
///
/// Returns:
/// - OK 201 Created + Location.
/// - Err 400 BadRequest + Explanation.
/// - Err 409 Conflict + Location
/// - Err 500 InternalServerError + Explanation.
#[put("/streams", data = "<request>")]
pub async fn fetch(
    storage: Connection<Storage>,
    request: Json<FetchRequest<'_>>,
) -> Result<Created<&str>, FetchError> {
    let (name, url) = request
        .validate()
        .context("Validating a request")
        .map_err(|ref e| FetchError::BadRequest(format!("{e:#?}")))?;

    match storage
        .streams()
        .find(name, &url)
        .await
        .context("Looking up existing streams")
        .map_err(|ref error| FetchError::ServerError(format!("Storage error: {error:#?}")))?
    {
        StreamDocumentMatch::None => {
            let (ref content_type, ref _content) = download(url.clone())
                .await
                .context("Fetching stream")
                .map_err(|ref e| FetchError::BadRequest(format!("{e:#?}")))?;

            if content_type.starts_with("application/vnd.apple.mpegurl") {
                // TODO - validate playlist.
                storage
                    .streams()
                    .add(name, &url)
                    .await
                    .context("Adding new stream")
                    .map(Created::new)
                    .map_err(|ref e| FetchError::ServerError(format!("{e:#}")))
            } else {
                Err(FetchError::ServerError(format!(
                    "Not supported content, type={content_type}"
                )))
            }
        }
        StreamDocumentMatch::NameMatch(doc) | StreamDocumentMatch::UrlMatch(doc) => {
            Err(FetchError::Conflict(Json(vec![doc.id()])))
        }
        StreamDocumentMatch::MultipleMatch(docs) => Err(FetchError::Conflict(Json(
            docs.into_iter().map(|d| d.id()).collect(),
        ))),
        StreamDocumentMatch::FullMatch(doc) => Ok(Created::new(doc.id())),
    }
}
