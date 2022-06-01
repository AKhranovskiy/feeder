use anyhow::Context;
use mongodb::bson::{doc, from_document};
use rocket::futures::TryStreamExt;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::msgpack::MsgPack;
use rocket_db_pools::Connection;

use crate::internal::storage::{AudioDocument, Storage};

pub mod upload;

fn to_internal_server_error(error: anyhow::Error) -> status::Custom<String> {
    log::error!("Internal Server Error: {}", error.to_string());
    status::Custom(Status::InternalServerError, error.to_string())
}

/// List of all audio segments, ordered by insertion date descending, with attached matches.
#[get("/segments", format = "msgpack")]
pub async fn segments(
    storage: Connection<Storage>,
) -> Result<status::Custom<MsgPack<Vec<AudioDocument>>>, status::Custom<String>> {
    let docs = storage
        .database("feeder")
        .collection::<AudioDocument>("audio")
        .aggregate([doc! {"$sample": { "size": 50 } }], None)
        .await
        .context("Aggregating")
        .map_err(to_internal_server_error)?
        .map_ok(|d| from_document::<AudioDocument>(d).unwrap())
        .try_collect::<Vec<_>>()
        .await
        .context("Collecting results")
        .map_err(to_internal_server_error)?;

    Ok(status::Custom(Status::Ok, MsgPack(docs)))
}
