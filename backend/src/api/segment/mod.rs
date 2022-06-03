use model::Tags;
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
            let content_type = get_content_type(&doc.tags);
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

fn get_content_type(tags: &Tags) -> ContentType {
    tags.get(&"FileType".to_string())
        .and_then(|file_type| ContentType::parse_flexible(file_type))
        .unwrap_or(ContentType::Binary)
}
