pub mod analyse;
pub mod metadata;
pub mod update;

use mongodb::bson::{doc, Uuid};
use rocket::http::ContentType;
use rocket_db_pools::Connection;

use crate::internal::codec::prepare_for_browser;
use crate::internal::storage::{AudioDocument, Storage};

#[get("/segment/<id>/audio")]
pub async fn segment_audio(
    id: &str,
    storage: Connection<Storage>,
) -> Option<(ContentType, Vec<u8>)> {
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
            let content = match prepare_for_browser(&content_type, &doc.content) {
                Ok(bytes) => bytes.to_vec(),
                Err(e) => {
                    log::error!("Failed remux aac: {e:#}");
                    doc.content.to_vec()
                }
            };
            (content_type, content)
        })
}
