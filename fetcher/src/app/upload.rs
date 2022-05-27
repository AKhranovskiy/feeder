use anyhow::{bail, Context};
use model::Segment;
use reqwest::multipart::{Form, Part};
use reqwest::{Client, StatusCode, Url};

pub async fn upload(segment: Segment) -> anyhow::Result<()> {
    let endpoint = "http://localhost:3456/api/v1/segment/insert"
        .parse::<Url>()
        .unwrap();

    let form = Form::new()
        .text("json", serde_json::to_string(&segment)?)
        .part(
            "content",
            Part::stream(segment.content.unwrap_or_default())
                .mime_str("application/octet-stream")
                .context("Attaching content")?,
        );
    let response = Client::new()
        .post(endpoint)
        .multipart(form)
        .send()
        .await
        .context("Sending reqwest")?;

    if response.status() == StatusCode::OK {
        Ok(())
    } else {
        bail!("{} {}", response.status(), response.text().await?);
    }
}
