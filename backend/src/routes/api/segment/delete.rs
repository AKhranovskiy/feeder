use mongodb::bson::doc;
use rocket::delete;
use rocket::http::Status;
use rocket_db_pools::Connection;

use crate::internal::emysound::delete_segment;
use crate::internal::storage::{AudioDocument, MatchDocument, MetadataDocument, Storage};

#[delete("/segment/<id>")]
pub async fn delete(storage: Connection<Storage>, id: &str) -> (Status, String) {
    log::debug!("Delete {id}");

    let uuid = mongodb::bson::Uuid::parse_str(id);

    if let Err(ref error) = uuid {
        log::error!("Invalid id, id={id}, error={error}");
        return (
            Status::BadRequest,
            format!("Invalid id, id={id}, error={error}"),
        );
    }

    let filter = doc! {"id": uuid.unwrap()};

    let db = storage.database("feeder");

    log::debug!("Deleting matches");

    let r = db
        .collection::<MatchDocument>("matches")
        .delete_many(filter.clone(), None)
        .await;

    if let Err(ref error) = r {
        log::error!("Failed to delete metadata, id={id}, error={error}");
        return (
            Status::InternalServerError,
            format!("Failed to delete metadata, id={id}, error={error}"),
        );
    }

    let deleted_matches = r.unwrap().deleted_count;

    log::debug!("Deleted {} matches", deleted_matches);

    log::debug!("Deleting audio");

    if let Err(ref error) = db
        .collection::<AudioDocument>("audio")
        .delete_many(filter.clone(), None)
        .await
    {
        log::error!("Failed to delete audio, id={id}, error={error}");
        return (
            Status::InternalServerError,
            format!("Failed to delete audio, id={id}, error={error}"),
        );
    }

    log::debug!("Deleting metadata");

    if let Err(ref error) = db
        .collection::<MetadataDocument>("metadata")
        .delete_many(filter.clone(), None)
        .await
    {
        log::error!("Failed to delete metadata, id={id}, error={error}");
        return (
            Status::InternalServerError,
            format!("Failed to delete metadata, id={id}, error={error}"),
        );
    }

    log::debug!("Deleting fingerprints");

    if let Err(ref error) = delete_segment(id).await {
        log::error!("Failed to delete fingerpints, id={id}, error={error}");
        // Tolerate fingerprint errors.
    }

    (Status::Ok, format!("Deleted {} matches", deleted_matches))
}
