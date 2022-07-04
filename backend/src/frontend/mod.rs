use mongodb::bson::{doc, Uuid};
use rocket::Route;
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};

use crate::internal::storage::{MetadataDocument, Storage};

pub fn routes() -> Vec<Route> {
    routes![index, view]
}

#[get("/")]
fn index() -> Template {
    Template::render("index", context! {})
}

#[get("/view/<id>")]
async fn view(id: &str, storage: Connection<Storage>) -> Option<Template> {
    let id = Uuid::parse_str(id).ok()?;
    let doc = storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata")
        .find_one(doc!["id": id], None)
        .await
        .ok()??;

    Some(Template::render("view", context! { data: doc}))
}
