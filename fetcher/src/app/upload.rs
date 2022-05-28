use anyhow::{bail, Context};
use bytes::Buf;
use model::{Segment, SegmentUploadResponse};
use reqwest::header::ACCEPT;
use reqwest::{header::CONTENT_TYPE, Client, StatusCode, Url};
use rmp_serde::Serializer;
use serde::Serialize;

const MSGPACK_MIME: &str = "application/msgpack";

pub async fn upload(endpoint: &Url, segment: Segment) -> anyhow::Result<SegmentUploadResponse> {
    let endpoint = endpoint.join("segments/upload")?;

    let mut payload = Vec::new();
    segment.serialize(&mut Serializer::new(&mut payload))?;

    let response = Client::new()
        .post(endpoint)
        .header(ACCEPT, MSGPACK_MIME)
        .header(CONTENT_TYPE, MSGPACK_MIME)
        .body(payload)
        .send()
        .await
        .context("Sending reqwest")?;

    match response.status() {
        StatusCode::OK | StatusCode::CREATED => {
            Ok(rmp_serde::from_read(response.bytes().await?.reader())?)
        }
        _ => {
            bail!("{} {}", response.status(), response.text().await?);
        }
    }
}
