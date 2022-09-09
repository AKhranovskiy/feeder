use rocket::get;
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};

use crate::internal::storage::Storage;
use crate::routes::api::segments::search::{search_raw, SearchQuery, SearchResult};

#[get("/search?<skip>&<limit>&<query..>")]
pub async fn search(
    storage: Connection<Storage>,
    query: SearchQuery<'_>,
    skip: Option<u64>,
    limit: Option<i64>,
) -> Template {
    let result = search_raw(storage, query.clone(), skip, limit)
        .await
        .or_else(|e| {
            log::error!("Search failed, query={query:#?}: {e:#?}");
            anyhow::Ok(SearchResult::error(e.to_string()))
        })
        .expect("Error must be converted to SearchResult");
    Template::render("search", context! { result: result })
}
