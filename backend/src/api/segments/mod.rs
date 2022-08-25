use anyhow::Context;
use futures::TryStreamExt;
use model::{ContentKind, MetadataWithAudio};
use mongodb::bson::{doc, from_document};
use mongodb::options::FindOptions;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::serde::msgpack::MsgPack;
use rocket_db_pools::Connection;

use crate::internal::documents::MetadataWithAudioDocument;
use crate::internal::storage::{MetadataDocument, Storage};

use self::query_options::QueryOptions;

use super::MetadataResponse;

pub mod delete;
mod query_options;
pub mod reasses;
pub mod search;
pub mod upload;

fn to_internal_server_error(error: anyhow::Error) -> status::Custom<String> {
    log::error!("Internal Server Error: {error:#}");
    status::Custom(Status::InternalServerError, error.to_string())
}

// TODO - move to metadata module.
#[get("/segments/json?<opts..>", format = "json")]
pub async fn segments_json(
    storage: Connection<Storage>,
    opts: QueryOptions<'_>,
) -> Result<status::Custom<Json<Vec<MetadataResponse>>>, status::Custom<String>> {
    let docs = storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata")
        .find(
            None,
            FindOptions::builder()
                .sort(doc! {"date_time": -1})
                .skip(opts.skip)
                .limit(opts.limit)
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
        .map(MetadataResponse::from)
        .collect::<Vec<_>>();

    Ok(status::Custom(Status::Ok, Json(docs)))
}

// TODO - move to metadata module.
#[get("/segments/msgpack?<opts..>", format = "msgpack")]
pub async fn segments_msgpack(
    storage: Connection<Storage>,
    opts: QueryOptions<'_>,
) -> Result<status::Custom<MsgPack<Vec<MetadataWithAudio>>>, status::Custom<String>> {
    let mut pipeline = Vec::new();

    if let Some(kind) = opts.kind {
        let _ = ContentKind::try_from(kind).map_err(to_internal_server_error)?;
        pipeline.push(doc! {
            "$match": doc! {
                "kind": kind
            }
        })
    }

    pipeline.push(doc! {
        "$lookup": doc! {
            "from": "audio",
            "localField": "id",
            "foreignField": "id",
            "as": "audio"
        }
    });

    pipeline.push(doc! {"$sort": doc! {"date_time": 1}});
    pipeline.push(doc! {"$project": doc! {"tags": 0, "date_time": 0}});

    if let Some(skip) = opts.skip {
        pipeline.push(doc! { "$skip": skip as i64});
    }

    if let Some(limit) = opts.limit {
        pipeline.push(doc! { "$limit": limit});
    }

    let docs = storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata")
        .aggregate(pipeline, None)
        .await
        .context("Aggregating")
        .map_err(to_internal_server_error)?
        .try_collect::<Vec<_>>()
        .await
        .context("Collecting results")
        .map_err(to_internal_server_error)?
        .into_iter()
        .map(from_document::<MetadataWithAudioDocument>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| to_internal_server_error(e.into()))?
        .into_iter()
        .map(MetadataWithAudio::from)
        .collect::<Vec<_>>();

    Ok(status::Custom(Status::Ok, MsgPack(docs)))
}
