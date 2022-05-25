use anyhow::{bail, Context};
use bytes::Bytes;
use reqwest::header::CONTENT_TYPE;
use reqwest::{StatusCode, Url};

pub async fn download(url: &Url) -> anyhow::Result<(Option<String>, Bytes)> {
    let response = reqwest::get(url.clone()).await.context("Fetching {url}")?;

    if response.status() != StatusCode::OK {
        bail!(
            "Failed to fetch {url}: {} {}",
            response.status(),
            response.text().await?
        );
    }

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_owned());

    let result = response
        .bytes()
        .await
        .map(|content| (content_type, content))?;

    Ok(result)
}
