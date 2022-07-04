use mongodb::bson::{doc, Uuid};
use rocket::serde::json::Json;
use rocket_db_pools::Connection;

use crate::api::MetadataResponse;
use crate::internal::storage::{MetadataDocument, Storage};

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
