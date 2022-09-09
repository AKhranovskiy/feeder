use anyhow::bail;
use futures::StreamExt;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let source = std::env::args().nth(1).expect("Expected source URL");

    println!("Fetching {source}...");

    let mut stream = hls_fetcher::fetch(source).await;

    while let Some(item) = stream.next().await {
        match item {
            Ok(segment) => println!(
                "Fetched segment: title={}, duration={}s, type={}, length={}",
                segment.comment,
                segment.duration.as_secs(),
                segment.content_type,
                segment.content.len()
            ),
            Err(error) => bail!(error),
        }
    }

    Ok(())
}
