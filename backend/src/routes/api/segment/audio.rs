use mongodb::bson::{doc, Uuid};
use rocket::{get, http::ContentType};
use rocket_db_pools::Connection;

use crate::internal::{
    codec::prepare_for_browser,
    storage::{AudioDocument, Storage},
};

#[get("/segment/<id>/audio")]
pub async fn audio(id: &str, storage: Connection<Storage>) -> Option<(ContentType, Vec<u8>)> {
    let id = Uuid::parse_str(id).ok()?;
    storage
        .database("feeder")
        .collection::<AudioDocument>("audio")
        .find_one(doc!["id": id], None)
        .await
        .ok()?
        .map(|doc| {
            let content_type =
                ContentType::parse_flexible(&doc.r#type).unwrap_or(ContentType::Binary);
            prepare_for_browser(&content_type, &doc.content).unwrap_or((content_type, doc.content))
        })
}
