// Rocket's FromForm generates code with a warning. It could be already fixed in the latest version of Rocket.
#![allow(clippy::unnecessary_lazy_evaluations)]

use std::collections::HashMap;
use std::str::FromStr;

use bytes::Bytes;
use mongodb::bson::{doc, Uuid};
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::Route;
use rocket_db_pools::Connection;
use rocket_dyn_templates::tera;
use rocket_dyn_templates::{context, Template};

use crate::api::segment::analyse::{analyse_impl, download};
use crate::api::segments::search::{search_raw, SearchQuery, SearchResult};
use crate::internal::storage::{MetadataDocument, Storage};

pub fn routes() -> Vec<Route> {
    routes![index, analyse_get, analyse_post, view, search]
}

#[get("/")]
fn index() -> Template {
    Template::render("index", context! {})
}

#[get("/analyse")]
fn analyse_get() -> Template {
    Template::render("analyse", context! {})
}

#[derive(Debug, FromForm)]
struct SegmentAnalyseForm<'f> {
    file: Option<TempFile<'f>>,
    url: Option<&'f str>,
}

#[post("/analyse", data = "<data>")]
async fn analyse_post(
    storage: Connection<Storage>,
    data: Form<SegmentAnalyseForm<'_>>,
) -> Template {
    // log::info!("Segment Analyse Post, data={data:#?}");
    let error = |msg: &str| Template::render("analyse", context! { error: msg});

    let result = match (&data.file, &data.url) {
        (Some(temp_file), None) => {
            let content: Bytes = if let TempFile::Buffered { content } = temp_file {
                Bytes::from(content.as_bytes().to_vec())
            } else if let TempFile::File { .. } = temp_file {
                let b = temp_file.path().and_then(|p| std::fs::read(p).ok());
                if b.is_none() {
                    return error("Failed to read file.");
                }
                b.unwrap().into()
            } else {
                unreachable!("Unknown TempFile variant")
            };
            analyse_impl(storage, &content).await
        }
        (None, Some(url)) => {
            let content = match reqwest::Url::from_str(url) {
                Ok(url) => match download(&url).await {
                    Ok(content) => content,
                    Err(e) => return error(&format!("Failed to download file from URL: {e:#?}")),
                },
                Err(e) => return Template::render("analyse", context! { error: e.to_string() }),
            };
            analyse_impl(storage, &content).await
        }
        (Some(_), Some(_)) => return error("File and url cannot be both specified"),
        (None, None) => return Template::render("analyse", context! {}),
    };

    match result {
        Ok(res) => {
            log::info!("Res: {res:?}");
            Template::render(
                "analyse",
                context! {
                    tags: res.tags,
                    content_kind_from_tags: res.content_kind_from_tags.to_string(),
                    fingerprints: res.fingerprints,
                    predictions: res.nn_predictions,
                },
            )
        }
        Err(e) => error(&format!("Failed to analyse: {e:#?}")),
    }
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

#[get("/search?<skip>&<limit>&<query..>")]
async fn search(
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

pub fn extend_tera(tera: &mut tera::Tera) {
    tera.register_function("contains", contains)
}

fn contains(args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let err = || {
        Err(tera::Error::msg(format!(
            "Invalid arguments, expected `values=[string], value=string`, given: {args:?}"
        )))
    };

    match (args.get("values"), args.get("value")) {
        (Some(values), Some(value)) => match (
            tera::from_value::<Vec<String>>(values.clone()),
            tera::from_value::<String>(value.clone()),
        ) {
            (Ok(values), Ok(value)) => {
                Ok(values.iter().any(|v| v.eq_ignore_ascii_case(&value)).into())
            }
            _ => err(),
        },
        _ => err(),
    }
}
