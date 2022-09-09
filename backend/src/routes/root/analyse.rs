// Rocket's FromForm generates code with a warning. It could be already fixed in the latest version of Rocket.
#![allow(clippy::unnecessary_lazy_evaluations)]

use std::str::FromStr;

use rocket::{form::Form, fs::TempFile, get, post, FromForm};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};
use url::Url;

use crate::internal::{download, storage::Storage};
use crate::routes::api::segment::analyse::analyse_impl;

#[get("/analyse")]
pub fn analyse_get() -> Template {
    Template::render("analyse", context! {})
}

#[derive(Debug, FromForm)]
pub struct SegmentAnalyseForm<'f> {
    file: Option<TempFile<'f>>,
    url: Option<&'f str>,
}

#[post("/analyse", data = "<data>")]
pub async fn analyse_post(
    storage: Connection<Storage>,
    data: Form<SegmentAnalyseForm<'_>>,
) -> Template {
    let error = |msg: &str| Template::render("analyse", context! { error: msg});

    let result = match (&data.file, &data.url) {
        (Some(temp_file), None) => {
            let content = if let TempFile::Buffered { content } = temp_file {
                content.as_bytes().to_vec()
            } else if let TempFile::File { .. } = temp_file {
                let b = temp_file.path().and_then(|p| std::fs::read(p).ok());
                if b.is_none() {
                    return error("Failed to read file.");
                }
                b.unwrap()
            } else {
                unreachable!("Unknown TempFile variant")
            };
            analyse_impl(storage, &content).await
        }
        (None, Some(url)) => {
            let content = match Url::from_str(url) {
                Ok(url) => match download(url).await {
                    Ok((_, content)) => content,
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
