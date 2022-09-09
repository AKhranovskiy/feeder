use anyhow::Context;
use model::ContentKind;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use rocket::futures::TryStreamExt;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket_db_pools::Connection;

use crate::api::segments::AudioDocumentLightweight;
use crate::internal::storage::{AudioDocument, Storage};

use super::to_internal_server_error;

#[derive(Debug, FromForm)]
pub struct Options {
    skip: Option<u64>,
    limit: Option<i64>,
}

#[get("/segments/kind/<kind>?<opt..>")]
pub async fn kind(
    kind: &str,
    opt: Options,
    storage: Connection<Storage>,
) -> Result<status::Custom<Json<Vec<AudioDocumentLightweight>>>, status::Custom<String>> {
    let kind = ContentKind::try_from(kind).map_err(|e| {
        log::error!("{e:#}");
        status::Custom(Status::NotFound, e.to_string())
    })?;

    log::info!("kind={kind:?}, opt={opt:?}");

    let docs = storage
        .database("feeder")
        .collection::<AudioDocument>("audio")
        .find(
            doc!["kind": kind.to_string()],
            FindOptions::builder()
                .skip(opt.skip)
                .limit(opt.limit)
                .sort(doc!["date_time": -1])
                .build(),
        )
        .await
        .context("Aggregating")
        .map_err(to_internal_server_error)?
        .try_collect::<Vec<_>>()
        .await
        .context("Collecting results")
        .map_err(to_internal_server_error)?
        .iter()
        .map(AudioDocumentLightweight::from)
        .collect::<Vec<_>>();
    Ok(status::Custom(Status::Ok, Json(docs)))
}
