// Rocket's FromForm generates code with a warning. It could be already fixed in the latest version of Rocket.
#![allow(clippy::unnecessary_lazy_evaluations)]

use anyhow::Context;
use futures::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::options::FindOptions;
use rocket::http::uri::fmt::{FromUriParam, Query, UriDisplay};
use rocket::serde::json::Json;
use rocket_db_pools::Connection;
use serde::Serialize;

use crate::api::MetadataResponse;
use crate::internal::storage::{MetadataDocument, Storage};

#[derive(Debug, Clone, FromForm, Serialize, Default)]
pub struct SearchQuery<'r> {
    pub id: Option<&'r str>,
    pub artist: Option<&'r str>,
    pub title: Option<&'r str>,
    pub kind: Vec<&'r str>,
    pub tags: Option<&'r str>,
}

impl ToString for SearchQuery<'_> {
    fn to_string(&self) -> String {
        format!("{}", self as &dyn UriDisplay<Query>)
    }
}

impl UriDisplay<Query> for SearchQuery<'_> {
    fn fmt(&self, f: &mut rocket::http::uri::fmt::Formatter<'_, Query>) -> std::fmt::Result {
        if let Some(id) = self.id.filter(|s| !s.is_empty()) {
            f.write_named_value("id", id)?;
        }

        if let Some(artist) = self.artist.filter(|s| !s.is_empty()) {
            f.write_named_value("artist", artist)?;
        }

        if let Some(title) = self.title.filter(|s| !s.is_empty()) {
            f.write_named_value("title", title)?;
        }

        for kind in self.kind.iter().filter(|s| !s.is_empty()) {
            f.write_named_value("kind", kind)?;
        }

        if let Some(tags) = self.tags.filter(|s| !s.is_empty()) {
            f.write_named_value("tags", tags)?;
        }

        Ok(())
    }
}

impl<'a> FromUriParam<Query, SearchQuery<'a>> for SearchQuery<'a> {
    type Target = SearchQuery<'a>;

    fn from_uri_param(param: SearchQuery<'a>) -> Self::Target {
        param
    }
}

#[derive(Debug, Serialize, Default)]
pub struct SearchResult<'q> {
    pub error: String,
    pub items: Vec<MetadataResponse>,
    pub total: u64,
    pub query: SearchQuery<'q>,
    pub skipped: u64,
    pub limit: i64,
}

impl<'q> SearchResult<'q> {
    pub fn new(
        items: Vec<MetadataResponse>,
        total: u64,
        query: SearchQuery<'q>,
        skipped: u64,
        limit: i64,
    ) -> Self {
        Self {
            error: String::new(),
            items,
            total,
            query,
            skipped,
            limit,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            error,
            ..Self::default()
        }
    }
}

#[get("/segments/search?<skip>&<limit>&<query..>", format = "json")]
pub async fn search_json(
    storage: Connection<Storage>,
    query: SearchQuery<'_>,
    skip: Option<u64>,
    limit: Option<i64>,
) -> Json<SearchResult> {
    let result = search_raw(storage, query.clone(), skip, limit)
        .await
        .or_else(|e| {
            log::error!("Search failed, query={query:#?}: {e:#?}");
            anyhow::Ok(SearchResult::error(e.to_string()))
        })
        .expect("Error must be converted to SearchResult");

    Json(result)
}

pub async fn search_raw(
    storage: Connection<Storage>,
    query: SearchQuery<'_>,
    skip: Option<u64>,
    limit: Option<i64>,
) -> anyhow::Result<SearchResult<'_>> {
    log::debug!("Search query={query:?}, skip={skip:#?}, limit={limit:#?}");

    let filter: Option<Document> = (&query).into();
    if filter.is_none() {
        return Ok(SearchResult::default());
    }

    let collection = storage
        .database("feeder")
        .collection::<MetadataDocument>("metadata");

    let total = collection.count_documents(filter.clone(), None).await?;

    if total == 0 {
        return Ok(SearchResult::default());
    }

    let skip = skip.unwrap_or_default();
    let limit = limit.unwrap_or(50);
    let items = collection
        .find(
            filter,
            FindOptions::builder()
                .sort(doc! {"date_time": 1})
                .skip(skip)
                .limit(limit)
                .build(),
        )
        .await
        .context("Aggregating")?
        .try_collect::<Vec<_>>()
        .await
        .context("Collecting results")?
        .iter()
        .map(MetadataResponse::from)
        .collect();

    Ok(SearchResult::new(items, total, query, skip, limit))
}

impl<'a> From<&SearchQuery<'a>> for Option<Document> {
    fn from(query: &SearchQuery<'a>) -> Self {
        let id = query
            .id
            .filter(|s| !s.is_empty())
            .and_then(|id| mongodb::bson::Uuid::parse_str(id).ok())
            .map(|id| doc! {"id": id});

        let artist = query.artist.filter(|s| !s.is_empty()).map(|artist| {
            doc! {"artist":
            doc! {"$regex": artist, "$options": "i"}}
        });

        let title = query.title.filter(|s| !s.is_empty()).map(|title| {
            doc! {"title":
            doc! {"$regex": title, "$options": "i"}}
        });

        let mut kinds = query
            .kind
            .iter()
            .filter(|s| !s.is_empty())
            .map(|kind| doc! {"kind": doc!{"$regex": kind, "$options": "i"}})
            .collect::<Vec<_>>();

        let kinds = match kinds.len() {
            0 => None,
            1 => kinds.pop(),
            _ => Some(doc! {"$or": kinds}),
        };

        let tags = query.tags.filter(|s| !s.is_empty()).map(|tags| {
            doc! {
            "$or": [
                doc!{"tags.TrackArtist": doc!{"$regex": tags, "$options": "i"}},
                doc!{"tags.TrackTitle": doc!{"$regex": tags, "$options": "i"}},
                doc!{"tags.Comment": doc!{"$regex": tags, "$options": "i"}},
                doc!{"tags.URL": doc!{"$regex": tags, "$options": "i"}},
                doc!{"tags.TXXX": doc!{"$regex": tags, "$options": "i"}},
                doc!{"tags.WXXX": doc!{"$regex": tags, "$options": "i"}},
                doc!{"tags.PRIV": doc!{"$regex": tags, "$options": "i"}},
            ]}
        });

        let mut clauses = [id, artist, title, kinds, tags]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        match clauses.len() {
            0 => None,
            1 => clauses.pop(),
            _ => Some(doc! {"$and": clauses}),
        }
    }
}
