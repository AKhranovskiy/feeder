use anyhow::Context;
use rocket::delete;
use rocket::response::status::NoContent;
use rocket_db_pools::Connection;

use crate::internal::storage::Storage;
use crate::storage::StorageScheme;

#[delete("/streams/<id>")]
pub async fn delete(storage: Connection<Storage>, id: &str) -> Option<NoContent> {
    storage
        .streams()
        .delete(id.into())
        .await
        .context("Deleting stream by id")
        .map(|deleted| if deleted { Some(NoContent) } else { None })
        .unwrap_or_else(|ref error| {
            log::error!("{error:#?}");
            None
        })
}
