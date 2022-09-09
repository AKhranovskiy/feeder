use mongodb::bson::{doc, Uuid};
use rocket::get;
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};

use crate::internal::storage::{MetadataDocument, Storage};

#[get("/view/<id>")]
pub async fn view(id: &str, storage: Connection<Storage>) -> Option<Template> {
    let id = Uuid::parse_str(id).ok()?;
    let doc = storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata")
        .find_one(doc!["id": id], None)
        .await
        .ok()??;

    Some(Template::render("view", context! { data: doc}))
}
