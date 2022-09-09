use std::ops::Range;

use anyhow::{anyhow, Context};
use rocket::data::{ByteUnit, ToByteUnit};
use url::Url;

// TODO - it is likely to be removed in favor of hls-fetcher. or moved to some shared crate.
pub async fn download(url: Url) -> anyhow::Result<(String, Vec<u8>)> {
    let valid_length_range: Range<ByteUnit> = 1.bytes()..2.mebibytes();

    tokio::task::spawn_blocking(move || {
        ureq::get(url.as_str())
            .call()
            .context(format!("Requesting {url}"))
            .and_then(|res| {
                res.header("Content-Length")
                    .and_then(|s| s.parse::<usize>().ok())
                    .filter(|v| valid_length_range.contains(v))
                    .ok_or_else(|| anyhow!("Invalid Content-Length"))
                    .and_then(|len| {
                        let content_type = res.content_type().to_owned();

                        let mut buf = vec![0x0; len];

                        res.into_reader()
                            .read_exact(&mut buf)
                            .map(|_| (content_type, buf))
                            .map_err(|e| e.into())
                    })
            })
    })
    .await
    .unwrap_or_else(|e| Err(anyhow!("Panic at the disco: {e:#?}")))
}
