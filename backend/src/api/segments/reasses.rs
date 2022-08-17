use anyhow::Context;
use futures::stream::TryStreamExt;
use model::ContentKind;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use rocket::response::stream::{Event, EventStream};
use rocket::tokio::select;
use rocket::Shutdown;
use rocket_db_pools::Connection;
use serde::Serialize;

use crate::internal::guess_content_kind;
use crate::internal::storage::{MetadataDocument, Storage};

#[derive(Debug, FromForm)]
pub struct ReassesContentKindOptions<'a> {
    update: Option<bool>,
    skip: Option<u64>,
    limit: Option<i64>,
    kind: Option<&'a str>,
}

#[patch("/segments/reasses?<options..>")]
pub async fn reasses_content_kind(
    options: ReassesContentKindOptions<'_>,
    storage: Connection<Storage>,
    mut shutdown: Shutdown,
) -> EventStream![] {
    let cursor = storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata")
        .find(
            options.kind.map(|kind| doc! {"kind": kind}),
            FindOptions::builder()
                .sort(doc! {"date_time": -1})
                .skip(options.skip)
                .limit(options.limit)
                .build(),
        )
        .await
        .context("Aggregating");

    let update = options.update.unwrap_or(false);

    EventStream! {
        if let Err(e) = cursor {
            log::error!("Reasses failed: {e:#?}");
            yield Event::data(e.to_string()).event("error");
            return;
        }

        let mut cursor = cursor.expect("Cursor is valid");

        let mut id = 0_usize;
        let mut updated = 0_usize;
        loop {
            select! {
                doc = cursor.try_next() => match doc {
                    Ok(Some(doc)) => {
                        let new_kind = guess_content_kind(&doc.tags);
                        if new_kind != doc.kind && update && let Err(e) = storage.database("feeder")
                        .collection::<MetadataDocument>("metadata")
                        .update_one(
                            doc! {"id": doc.id},
                            doc! {"$set": {"kind": new_kind.to_string()}},
                            None
                        )
                        .await
                        .context("Updating") {
                            log::error!("Update failed: {e:#?}");
                            yield Event::data(e.to_string()).event("error");

                            break;
                        }

                        if new_kind != doc.kind && update{
                            updated += 1;
                        }

                        yield Event::json(&ReassesContentKind{
                            id: uuid::Uuid::from_bytes(doc.id.bytes()),
                            artist: &doc.artist,
                            title: &doc.title,
                            // tags: &doc.tags,
                            current_kind: doc.kind,
                            new_kind,
                            updated: new_kind != doc.kind && update
                        }).id(id.to_string());
                    },
                    Ok(None) => {
                        yield Event::comment(format!("Complete. Updated {updated} docs.")).event("end");
                        break;
                    },
                    Err(e) => {
                        log::error!("Reasses failed: {e:#?}");
                        yield Event::data(e.to_string()).event("error");
                        break;
                    }
                },
                _ = &mut shutdown => {
                    yield Event::comment("Shutdown").event("end");
                    break;
                }
            }
            id += 1;

        }
    }
}

#[derive(Debug, Serialize)]
struct ReassesContentKind<'s> {
    id: uuid::Uuid,
    artist: &'s str,
    title: &'s str,
    // tags: &'s Tags,
    current_kind: ContentKind,
    new_kind: ContentKind,
    updated: bool,
}
