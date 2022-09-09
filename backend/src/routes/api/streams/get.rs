use anyhow::Context;
use futures::{TryFutureExt, TryStreamExt};
use rocket::get;
use rocket::serde::json::Json;
use rocket_db_pools::Connection;

use super::StreamData;
use crate::internal::storage::Storage;
use crate::storage::StorageScheme;

#[get("/streams/<id>")]
pub async fn get_one(storage: Connection<Storage>, id: &str) -> Option<Json<StreamData>> {
    storage
        .streams()
        .get(id.into())
        .await
        .context("Getting stream by id")
        .unwrap_or_else(|ref error| {
            log::error!("{error:#?}");
            None
        })
        .map(StreamData::from)
        .map(Json)
}

#[get("/streams")]
pub async fn get_all(storage: Connection<Storage>) -> Json<Vec<StreamData>> {
    let docs = storage
        .streams()
        .inner()
        .find(None, None)
        .and_then(|cursor| cursor.try_collect())
        .await
        .unwrap_or_else(|ref error| {
            log::error!("Storage failure: {error:#?}");
            vec![]
        })
        .into_iter()
        .map(StreamData::from)
        .collect();

    Json(docs)
}
