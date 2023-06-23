mod hls;

use std::io::Read;

use anyhow::bail;
use url::Url;

static MIME_AUDIO: &str = "audio/";

#[non_exhaustive]
pub struct Unstreamer(Box<dyn Read + Send>);

impl Unstreamer {
    pub fn open(source: &str) -> anyhow::Result<Unstreamer> {
        if let Ok(url) = Url::parse(source) {
            let resp = ureq::get(url.as_ref()).call()?;
            if resp.content_type() == hls::MIME_HLS {
                Ok(Self(Box::new(hls::HLSUnstreamer::open(url)?)))
            } else if resp.content_type().starts_with(MIME_AUDIO) {
                Ok(Self(resp.into_reader()))
            } else {
                bail!("Unsupported content type: {}", resp.content_type());
            }
        } else if let Ok(file) = std::fs::File::open(source) {
            Ok(Self(Box::new(file)))
        } else {
            bail!("Unsupported source: {}", source);
        }
    }
}

impl Read for Unstreamer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}
