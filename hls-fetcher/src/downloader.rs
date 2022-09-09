use anyhow::{anyhow, Context};
use async_trait::async_trait;

#[async_trait]
pub trait Downloader {
    async fn download(&self, source: String) -> anyhow::Result<(String, Vec<u8>)>;
}

pub struct UReqDl;

#[async_trait]
impl Downloader for UReqDl {
    async fn download(&self, source: String) -> anyhow::Result<(String, Vec<u8>)> {
        // TODO - extract to lib.
        const MEBIBYTE: usize = 1048576;
        let valid_length_range = 1..2 * MEBIBYTE;

        tokio::task::spawn_blocking(move || {
            ureq::get(source.as_str())
                .call()
                .context(format!("Requesting {source}"))
                .and_then(|res| {
                    res.header("Content-Length")
                        .and_then(|s| s.parse::<usize>().ok())
                        .filter(|v| valid_length_range.contains(v))
                        .ok_or_else(|| anyhow!("Invalid Content-Length"))
                        .and_then(|len| {
                            let content_type = res.content_type().to_owned();

                            let mut buf = vec![0; len];
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
}

#[cfg(test)]
pub struct TestDl {
    contents: Vec<(String, Vec<u8>)>,
    next: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
impl TestDl {
    pub fn new(contents: Vec<(String, Vec<u8>)>) -> Self {
        Self {
            contents,
            next: std::sync::Arc::new(0.into()),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl Downloader for TestDl {
    async fn download(&self, _source: String) -> anyhow::Result<(String, Vec<u8>)> {
        use std::sync::atomic::Ordering::{Relaxed, SeqCst};

        let next = self.next.load(Relaxed);
        if next >= self.contents.len() {
            anyhow::bail!(
                "Fail to download, next={next}, content len={}",
                self.contents.len()
            );
        } else {
            self.next.fetch_add(1, SeqCst);
            Ok(self.contents[next].clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Downloader, TestDl};

    #[tokio::test]
    async fn test_downloader() {
        let content_type = "plain/text".to_string();
        let contents: Vec<Vec<u8>> = ["ABC", "123", ""]
            .into_iter()
            .map(|s| s.as_bytes().to_vec())
            .collect();

        let downloader = TestDl::new(
            std::iter::repeat(content_type.clone())
                .zip(contents.iter().cloned())
                .collect(),
        );

        assert_eq!(
            (content_type.clone(), contents[0].clone()),
            downloader.download(String::new()).await.unwrap()
        );
        assert_eq!(
            (content_type.clone(), contents[1].clone()),
            downloader.download(String::new()).await.unwrap()
        );
        assert_eq!(
            (content_type.clone(), contents[2].clone()),
            downloader.download(String::new()).await.unwrap()
        );
        assert!(downloader.download(String::new()).await.is_err());
    }
}
