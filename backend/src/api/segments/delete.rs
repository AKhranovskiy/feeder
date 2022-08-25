// Rocket's FromForm generates code with a warning. It could be already fixed in the latest version of Rocket.
#![allow(clippy::unnecessary_lazy_evaluations)]

use mongodb::bson::doc;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket_db_pools::Connection;
use serde::Serialize;

use crate::internal::storage::Storage;

#[derive(Debug, Serialize)]
pub struct DeleteManyResult {
    requested_count: u64,
    deleted_count: u64,
    error: String,
    not_processed_ids: Vec<String>,
}

#[post("/segments/delete?<ignore_missing>", data = "<ids>")]
pub async fn delete(
    _storage: Connection<Storage>,
    ids: Json<Vec<&str>>,
    ignore_missing: Option<bool>,
) -> (Status, Json<DeleteManyResult>) {
    log::debug!(
        "Delete {} items, ignore_missing={ignore_missing:?}",
        ids.len()
    );

    (
        Status::NotImplemented,
        Json(DeleteManyResult {
            requested_count: ids.len() as u64,
            deleted_count: 0,
            error: "Not implemented".to_owned(),
            not_processed_ids: ids.iter().map(|&id| id.to_owned()).collect(),
        }),
    )
    // let uuid = mongodb::bson::Uuid::parse_str(id);

    // if let Err(ref error) = uuid {
    //     log::error!("Invalid id, id={id}, error={error}");
    //     return (
    //         Status::BadRequest,
    //         format!("Invalid id, id={id}, error={error}"),
    //     );
    // }

    // let filter = doc! {"id": uuid.unwrap()};

    // let db = storage.database("feeder");

    // log::debug!("Deleting matches");

    // let r = db
    //     .collection::<MatchDocument>("matches")
    //     .delete_many(filter.clone(), None)
    //     .await;

    // if let Err(ref error) = r {
    //     log::error!("Failed to delete metadata, id={id}, error={error}");
    //     return (
    //         Status::InternalServerError,
    //         format!("Failed to delete metadata, id={id}, error={error}"),
    //     );
    // }

    // let deleted_matches = r.unwrap().deleted_count;

    // log::debug!("Deleted {} matches", deleted_matches);

    // log::debug!("Deleting audio");

    // if let Err(ref error) = db
    //     .collection::<AudioDocument>("audio")
    //     .delete_many(filter.clone(), None)
    //     .await
    // {
    //     log::error!("Failed to delete audio, id={id}, error={error}");
    //     return (
    //         Status::InternalServerError,
    //         format!("Failed to delete audio, id={id}, error={error}"),
    //     );
    // }

    // log::debug!("Deleting metadata");

    // if let Err(ref error) = db
    //     .collection::<MetadataDocument>("metadata")
    //     .delete_many(filter.clone(), None)
    //     .await
    // {
    //     log::error!("Failed to delete metadata, id={id}, error={error}");
    //     return (
    //         Status::InternalServerError,
    //         format!("Failed to delete metadata, id={id}, error={error}"),
    //     );
    // }

    // log::debug!("Deleting fingerprints");

    // if let Err(ref error) = delete_segment(id).await {
    //     log::error!("Failed to delete fingerpints, id={id}, error={error}");

    //     // Tolerate fingerprint errors.
    //     // return (
    //     //     Status::InternalServerError,
    //     //     format!("Failed to delete fingerprints, id={id}, error={error}"),
    //     // );
    // }

    // (Status::Ok, format!("Deleted {} matches", deleted_matches))
}
