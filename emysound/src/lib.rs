use anyhow::{anyhow, ensure, Result};
use multipart::client::lazy::Multipart;
use url::Url;
use uuid::Uuid;

mod model;

pub use model::QueryResult;

const BASIC_AUTH: &str = "Basic QURNSU46";

pub fn check_connection(endpoint: &Url) -> Result<()> {
    let url = endpoint.join("Streams")?;
    ureq::get(url.as_str())
        .set("authorization", BASIC_AUTH)
        .call()?;

    Ok(())
}

pub fn insert(endpoint: &Url, id: &Uuid, artist: &str, title: &str, content: &[u8]) -> Result<()> {
    log::trace!(
        target: "emysound::insert",
        "endpoint={endpoint}, id={id}, artist={artist}, title={title}, content:len={}",
        content.len()
    );

    let id = id.to_string();
    let url = endpoint.join("Tracks")?;
    let filename = format!("{id}-{artist}-{title}");

    let mdata = {
        let mut m = Multipart::new();
        m.add_text("Id", &id);
        m.add_text("Artist", artist);
        m.add_text("Title", title);
        m.add_text("MediaType", "Audio");
        m.add_stream(
            "file",
            content,
            Some(filename),
            Some(mime::APPLICATION_OCTET_STREAM),
        );
        m.prepare().expect("Valid data")
    };

    ureq::post(url.as_str())
        .set("accept", "application/json")
        .set("authorization", BASIC_AUTH)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={}", mdata.boundary()),
        )
        .send(mdata)
        .map(|_| ())
        .map_err(|e| anyhow!("Failed to insert track: {e:#}"))
}

pub fn query(endpoint: &Url, content: &[u8], min_confidence: f32) -> Result<Vec<QueryResult>> {
    log::trace!(
        target: "emysound::query",
        "endpoint={endpoint}, content:len={}, min_confidence={min_confidence:.02}",
        content.len()
    );

    ensure!(
        (0f32..=1f32).contains(&min_confidence),
        "Min confidence must be between 0 and 1"
    );

    let mdata = {
        let mut m = Multipart::new();
        m.add_stream(
            "file",
            content,
            Some("query.file"),
            Some(mime::APPLICATION_OCTET_STREAM),
        );
        m.prepare().expect("Valid data")
    };

    let url = endpoint.join("Query")?;

    let resp = ureq::post(url.as_str())
        .set("accept", "application/json")
        .set("authorization", BASIC_AUTH)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={}", mdata.boundary()),
        )
        .query("mediaType", "Audio")
        .query("minCoverage", &min_confidence.to_string())
        .send(mdata)?;

    resp.into_json()
        .map_err(|e| anyhow!("Failed to query track: {e:#}"))
}

pub fn delete(endpoint: &Url, id: &Uuid) -> anyhow::Result<()> {
    log::trace!(target: "emysound::delete", "endpoint={endpoint}, id={id}");

    let id = id.to_string();
    let url = endpoint.join("Tracks/")?.join(&id)?;

    ureq::delete(url.as_str())
        .set("authorization", BASIC_AUTH)
        .call()
        .map_err(std::convert::Into::into)
        .map(|_| ())
}
