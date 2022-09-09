// Rocket's FromForm generates code with a warning. It could be already fixed in the latest version of Rocket.
#![allow(clippy::unnecessary_lazy_evaluations)]

use mongodb::bson::{doc, Uuid};
use rocket::get;
use rocket::serde::json::Json;
use rocket_db_pools::Connection;

use crate::internal::storage::{MetadataDocument, Storage};

use super::MetadataResponse;

#[get("/segment/<id>/metadata")]
pub async fn metadata(id: &str, storage: Connection<Storage>) -> Option<Json<MetadataResponse>> {
    let id = Uuid::parse_str(id).ok()?;
    storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata")
        .find_one(doc!["id": id], None)
        .await
        .ok()?
        .map(|doc| Json(MetadataResponse::from(&doc)))
}
