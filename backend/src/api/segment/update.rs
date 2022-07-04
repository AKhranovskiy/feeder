use model::ContentKind;
use mongodb::bson::{doc, Uuid};
use rocket::form::Form;
use rocket::response::status::{self, BadRequest};
use rocket_db_pools::Connection;

use crate::internal::storage::{MetadataDocument, Storage};

#[derive(Debug, FromForm)]
pub struct UpdateData<'r> {
    id: &'r str,
    kind: &'r str,
}

#[post("/segment/update", data = "<data>")]
pub async fn update(
    data: Form<UpdateData<'_>>,
    storage: Connection<Storage>,
) -> Result<status::NoContent, BadRequest<String>> {
    let id =
        Uuid::parse_str(data.id).map_err(|e| BadRequest(Some(format!("Invalid ID: {e:#}"))))?;

    let kind = ContentKind::try_from(data.kind)
        .map_err(|e| BadRequest(Some(format!("Invalid Kind: {e:#}"))))?;

    match storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata")
        .find_one_and_update(
            doc! { "id": id},
            doc! {"$set": {"kind": kind.to_string()}},
            None,
        )
        .await
        .map_err(|e| BadRequest(Some(format!("Update failed {e:#}"))))?
    {
        Some(_) => Ok(status::NoContent),
        None => Err(BadRequest(Some(format!("Not found, id={id}")))),
    }
}
